pub mod streaming {
    use gstreamer as gst;
    use gstreamer::prelude::*;
    use std::error::Error;
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use egui::Context;
    use crate::MyApp;
    use if_addrs::get_if_addrs;

    // Funzione per ottenere l'indirizzo IP locale
    fn get_local_ip() -> Result<String, Box<dyn std::error::Error>> {
        for iface in get_if_addrs()? {
            if iface.ip().is_ipv4() && !iface.is_loopback() {
                return Ok(iface.ip().to_string());
            }
        }
        Err("Nessun indirizzo IP valido trovato.".into())
    }

    fn handle_connection(stream: TcpStream, pipeline: Arc<Mutex<gst::Pipeline>>) -> Result<(), Box<dyn Error>> {
        let receiver_ip = stream.peer_addr()?.ip().to_string();
        println!("Nuova connessione da: {}", receiver_ip);

        {
            let pipeline_locked = pipeline.lock().unwrap();
            if let Some(sink) = pipeline_locked.by_name("udpsink") {
                // Aggiorna dinamicamente l'IP del sink
                sink.set_property("host", &receiver_ip);
            }
        }
        Ok(())
    }

    pub fn start_streaming(ctx: &Context, app: &mut MyApp) -> Result<(), Box<dyn Error>> {
        gst::init()?; // Inizializzazione di GStreamer

        // Ottieni l'indirizzo IP locale
        let local_ip = get_local_ip()?;
        println!("Indirizzo IP locale rilevato: {}", local_ip);

        // Usa una porta dinamica
        let listener = TcpListener::bind((local_ip.as_str(), 0))?;
        let port = 50496;
        println!("Server in ascolto su {}:{}", local_ip, port);

        let pipeline = gst::Pipeline::new(None);
        let shared_pipeline = Arc::new(Mutex::new(pipeline));

        // Creazione degli elementi della pipeline
        let src = gst::ElementFactory::make("videotestsrc")
            .name("src")
            .build()
            .expect("Elemento 'videotestsrc' non trovato");

        let capsfilter_src = gst::ElementFactory::make("capsfilter")
            .name("capsfilter_src")
            .build()
            .expect("Elemento 'capsfilter_src' non trovato");
        let caps_src = gst::Caps::builder("video/x-raw")
            .field("format", &"I420_10LE")
            .field("width", &1280)
            .field("height", &720)
            .field("framerate", &gst::Fraction::new(30, 1))
            .build();
        capsfilter_src.set_property("caps", &caps_src);

        let videoconvert = gst::ElementFactory::make("videoconvert")
            .name("videoconvert")
            .build()
            .expect("Elemento 'videoconvert' non trovato");

        let capsfilter_convert = gst::ElementFactory::make("capsfilter")
            .name("capsfilter_convert")
            .build()
            .expect("Elemento 'capsfilter_convert' non trovato");
        let caps_convert = gst::Caps::builder("video/x-raw")
            .field("format", &"I420")
            .field("width", &1280)
            .field("height", &720)
            .field("framerate", &gst::Fraction::new(30, 1))
            .build();
        capsfilter_convert.set_property("caps", &caps_convert);

        let encoder = gst::ElementFactory::make("x264enc")
            .name("encoder")
            .build()
            .expect("Elemento 'x264enc' non trovato");

        let rtp_payload = gst::ElementFactory::make("rtph264pay")
            .name("rtp_payload")
            .build()
            .expect("Elemento 'rtph264pay' non trovato");

        let udpsink = gst::ElementFactory::make("udpsink")
            .name("udpsink")
            .build()
            .expect("Elemento 'udpsink' non trovato");
        udpsink.set_property("host", &local_ip);
        udpsink.set_property("port", &(port as i32));

        let tee = gst::ElementFactory::make("tee")
            .name("tee")
            .build()
            .expect("Elemento 'tee' non trovato");

        {
            let mut pipeline_locked = shared_pipeline.lock().unwrap();
            pipeline_locked.add_many(&[
                &src,
                &capsfilter_src,
                &videoconvert,
                &capsfilter_convert,
                &encoder,
                &rtp_payload,
                &tee,
                &udpsink,
            ])?;

            gst::Element::link_many(&[
                &src,
                &capsfilter_src,
                &videoconvert,
                &capsfilter_convert,
                &encoder,
                &rtp_payload,
                &tee,
            ])?;

            let tee_src_pad_udpsink = tee
                .request_pad_simple("src_%u")
                .expect("Impossibile richiedere un pad al tee");
            let udpsink_sink_pad = udpsink
                .static_pad("sink")
                .expect("Pad sink non trovato in udpsink");
            tee_src_pad_udpsink.link(&udpsink_sink_pad)?;
        }

        {
            let pipeline_locked = shared_pipeline.lock().unwrap();
            match pipeline_locked.set_state(gst::State::Playing) {
                Ok(_) => println!("Pipeline avviata con successo."),
                Err(err) => {
                    eprintln!("Errore nell'avvio della pipeline: {}", err);
                    return Err(Box::new(err));
                }
            }
        }

        let bus_pipeline = Arc::clone(&shared_pipeline);
        thread::spawn(move || {
            let pipeline_locked = bus_pipeline.lock().unwrap();
            let bus = pipeline_locked.bus().expect("Pipeline senza bus.");
            for msg in bus.iter_timed(gst::ClockTime::NONE) {
                use gst::MessageView;
                match msg.view() {
                    MessageView::Eos(..) => {
                        println!("Fine dello stream.");
                        let mut pipeline_locked = bus_pipeline.lock().unwrap();
                        pipeline_locked.set_state(gst::State::Null).expect("Impossibile impostare la pipeline su NULL.");
                        break;
                    }
                    MessageView::Error(err) => {
                        eprintln!("Errore dalla pipeline: {}", err.error());
                        if let Some(debug) = err.debug() {
                            eprintln!("Debug: {}", debug);
                        }
                        let mut pipeline_locked = bus_pipeline.lock().unwrap();
                        pipeline_locked.set_state(gst::State::Null).expect("Impossibile impostare la pipeline su NULL.");
                        break;
                    }
                    MessageView::Warning(warn) => {
                        eprintln!("Avviso: {}", warn.error());
                    }
                    _ => (),
                }
            }
        });

        Ok(())
    }
}
