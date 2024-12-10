pub mod streaming {
    use gstreamer as gst;
    use gstreamer::prelude::*;
    use std::error::Error;
    use std::net::IpAddr;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use egui::Context;
    use if_addrs::get_if_addrs;
    use crate::MyApp;
    /// Funzione per ottenere l'indirizzo IP dell'interfaccia Wi-Fi
    fn get_wifi_ip() -> Option<IpAddr> {
        if let Ok(if_addrs) = get_if_addrs() {
            for iface in if_addrs {
               // println!("Interfaccia: {}", iface.name);
                //println!("IP: {:?}", iface.ip());
               // println!("Is loopback: {}", iface.ip().is_loopback());
                //println!("Is IPv4: {}", iface.ip().is_ipv4());

                // Modifica il criterio per scegliere la Wi-Fi
                if iface.ip().is_ipv4() && !iface.ip().is_loopback() {
                    // Puoi aggiungere altri controlli, come verificare la sottorete
                    return Some(iface.ip());
                }
            }
        }
        None
    }
    pub fn start_streaming(ctx: &Context, app: &mut MyApp) -> Result<(), Box<dyn Error>> {
        gst::init()?; // Inizializzazione di GStreamer

        // Ottieni l'indirizzo IP del Wi-Fi
        let mut wifi_ip = match get_wifi_ip() {
            Some(ip) => ip.to_string(),
            None => {
                eprintln!("Impossibile determinare l'indirizzo IP del Wi-Fi.");
                return Err("Impossibile determinare l'indirizzo IP del Wi-Fi.".into());
            }
        };
        println!("Indirizzo IP del Wi-Fi: {}", wifi_ip);
        wifi_ip="192.168.26.255".to_string();
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
            .field("format", &"I420") // Formato compatibile con encoder
            .field("width", &1280)
            .field("height", &720)
            .field("framerate", &gst::Fraction::new(30, 1))
            .build();
        capsfilter_src.set_property("caps", &caps_src);

        let capsfilter_rtp = gst::ElementFactory::make("capsfilter")
            .name("capsfilter_rtp")
            .build()
            .expect("Elemento 'capsfilter_rtp' non trovato");

        let caps_rtp = gst::Caps::builder("application/x-rtp")
            .field("media", &"video")
            .field("encoding-name", &"H264")
            .field("payload", &96) // Payload type usato dal receiver
            .build();
        capsfilter_rtp.set_property("caps", &caps_rtp);

        let videoconvert = gst::ElementFactory::make("videoconvert")
            .name("videoconvert")
            .build()
            .expect("Elemento 'videoconvert' non trovato");

        let encoder = gst::ElementFactory::make("x264enc")
            .name("encoder")
            .build()
            .expect("Elemento 'x264enc' non trovato");

        let rtp_payload = gst::ElementFactory::make("rtph264pay")
            .name("rtp_payload")
            .build()
            .expect("Elemento 'rtph264pay' non trovato");
        rtp_payload.set_property("pt", &96u32);

        let udpsink = gst::ElementFactory::make("udpsink")
            .name("udpsink")
            .build()
            .expect("Elemento 'udpsink' non trovato");

        // Configurazione dell'indirizzo di broadcast e porta
        udpsink.set_property("host", wifi_ip.clone()); // Broadcast su rete locale
        udpsink.set_property("port", &50496i32); // Porta predefinita per ricevere il flusso

        // Configura la pipeline
        {
            let mut pipeline_locked = shared_pipeline.lock().unwrap();
            pipeline_locked.add_many(&[
                &src,
                &capsfilter_src,
                &videoconvert,
                &encoder,
                &rtp_payload,
                &udpsink,
                &capsfilter_rtp,
            ])?;

            gst::Element::link_many(&[
                &src,
                &capsfilter_src,
                &videoconvert,
                &encoder,
                &rtp_payload,
                &capsfilter_rtp,
                &udpsink,
            ])?;
        }

        // Avvio della pipeline
        {
            let pipeline_locked = shared_pipeline.lock().unwrap();
            match pipeline_locked.set_state(gst::State::Playing) {
                Ok(_) => println!("Streaming avviato con successo su {}:50496",wifi_ip.clone()),
                Err(err) => {
                    eprintln!("Errore nell'avvio dello streaming: {}", err);
                    return Err(Box::new(err));
                }
            }
        }

        // Gestione del bus per log e errori
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
