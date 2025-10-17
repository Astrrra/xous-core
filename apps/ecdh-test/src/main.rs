#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

use log::{error, info};
use num_traits::ToPrimitive;

use gam::UxRegistration;
use xous_ipc::Buffer;

mod ui;
use ui::EcdhTestUi;

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
pub enum EcdhTestOp {
    /// Redraw UI
    Redraw = 0,
    /// User input line
    Line = 1,
    /// Focus change event
    ChangeFocus = 2,
    /// Quit the app
    Quit = 3,
}

const SERVER_NAME_ECDH_TEST: &str = "_ECDH Test App_";

fn main() -> ! {
    log_server::init_wait().unwrap();
    info!("ECDH Test App starting...");

    let xns = xous_names::XousNames::new().unwrap();
    let sid = xns.register_name(SERVER_NAME_ECDH_TEST, None).expect("can't register server");

    // Register with GAM
    let gam = gam::Gam::new(&xns).expect("can't connect to GAM");

    let token = gam
        .register_ux(UxRegistration {
            app_name: String::from("ecdh-test"),
            ux_type: gam::UxType::Chat,
            predictor: Some(String::from(ime_plugin_shell::SERVER_NAME_IME_PLUGIN_SHELL)),
            listener: sid.to_array(),
            redraw_id: EcdhTestOp::Redraw.to_u32().unwrap(),
            gotinput_id: Some(EcdhTestOp::Line.to_u32().unwrap()),
            audioframe_id: None,
            rawkeys_id: None,
            focuschange_id: Some(EcdhTestOp::ChangeFocus.to_u32().unwrap()),
        })
        .expect("couldn't register Ux context");

    if token.is_none() {
        error!("GAM register_ux returned None - GAM might not be ready yet");
        xous::terminate_process(1)
    }

    // Create UI handler
    let mut ui = EcdhTestUi::new(&xns, &gam, token.unwrap());

    ui.add_message("ECDH Test App v0.1.0");
    ui.add_message("Type 'help' for commands");
    ui.redraw().expect("couldn't do initial redraw");

    info!("ECDH Test App ready, entering main loop");

    loop {
        let msg = xous::receive_message(sid).unwrap();
        match num_traits::FromPrimitive::from_usize(msg.body.id()) {
            Some(EcdhTestOp::Redraw) => {
                ui.redraw().ok();
            }
            Some(EcdhTestOp::Line) => {
                let buffer = unsafe { Buffer::from_memory_message(msg.body.memory_message().unwrap()) };
                let s = buffer.as_flat::<String, _>().unwrap();
                let input = s.as_str();

                info!("Received input: {}", input);
                ui.handle_input(input);
                ui.redraw().ok();
            }
            Some(EcdhTestOp::ChangeFocus) => {
                // Focus change - we don't need to do anything special
            }
            Some(EcdhTestOp::Quit) => {
                info!("Quit requested, exiting");
                break;
            }
            None => {
                error!("Unknown opcode: {}", msg.body.id());
            }
        }
    }

    info!("ECDH Test App exiting");
    xous::terminate_process(0)
}
