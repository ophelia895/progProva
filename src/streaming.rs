use gstreamer::prelude::*;
use gstreamer::ElementFactory;
use std::error::Error;
use gstreamer_app::gst;

pub fn start_server() -> Result<(), Box<dyn std::error::Error>> {
    // Inizializza GStreamer
    gst::init()?;
    let host_ip = "192.168.216.246";  // Cambia con l'IP del server
    let pipeline_str = format!(
        "d3d11screencapturesrc ! videoconvert ! x264enc tune=zerolatency bitrate=3000 speed-preset=ultrafast ! \
    rtph264pay config-interval=1 pt=96 ! udpsink host={} port=5000",
        host_ip
    );
    let pipeline = gst::parse_launch(&pipeline_str)?;

    let pipeline = pipeline
        .downcast::<gst::Pipeline>()
        .expect("La pipeline non è valida");

    // Avvia la pipeline
    pipeline.set_state(gst::State::Playing)?;

    println!("Server UDP in esecuzione su 127.0.0.1:5000...");

    // Mantieni il server attivo
    let bus = pipeline.bus().unwrap();
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        match msg.view() {
            gst::MessageView::Error(err) => {
                eprintln!("Errore: {:?}", err);
                break;
            }
            gst::MessageView::Eos(..) => {
                println!("Fine dello streaming");
                break;
            }
            _ => {}
        }
    }

    // Ferma la pipeline
    pipeline.set_state(gst::State::Null)?;
    Ok(())
}
