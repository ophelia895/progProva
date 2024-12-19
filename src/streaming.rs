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
    use gstreamer::prelude::*;
    use gstreamer_video::VideoEncoder;

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
        wifi_ip="127.0.0.1".to_string();

        let pipeline = gst::Pipeline::new(None);
        let shared_pipeline = Arc::new(Mutex::new(pipeline));
        // rtmp uri
        let rtmp_uri = "rtmp://localhost:50496/live";
        // Creazione degli elementi della pipeline
        let src = gst::ElementFactory::make("videotestsrc")
            .name("src")
            .build()
            .expect("Elemento 'videotestsrc' non trovato"); // create gstreamer elements
         /*  //encoder
        let x264enc = gst::ElementFactory::make("x264enc")
            .name("encoder")
            .build()
            .expect("Elemento 'x264enc' non trovato");
        let bitrate: u32 = 500; // esempio: 500 kbps
        x264enc.set_property("bitrate", bitrate);// Bitrate in kbps
        x264enc.set_property_from_str("speed-preset", "ultrafast"); // Impostazione per debug
        /*
        let x264enc = gst::ElementFactory::make("x264enc", Some("x264enc"))
            .expect("Could not create sink element x264enc");*/
        let flvmux = gst::ElementFactory::make("flvmux").name("flvmux")
            .build().expect("ELEMENTO flvmux non trovato");

        let video_sink = gst::ElementFactory::make("rtmpsink").name("rtmpsink").build().expect("Elemnet rtmpsink non trovato");

        // set properties
        //video_sink.set_property("location", true);
        video_sink.set_property_from_str("location", rtmp_uri);

        // Create empty pipeline
        let pipeline = gst::Pipeline::new(Some("live-pipeline"));

        // Build the pipeline
        pipeline.add_many(&[&source, &x264enc, &flvmux, &video_sink]).unwrap();
        gst::Element::link_many(&[&source, &x264enc, &flvmux, &video_sink])
            .expect("Elements could not be linked!");

        // start playing
        pipeline.set_state(gst::State::Playing)
            .expect("Unable to set the pipeline playing state");

        // Wait until error or EOS
        let bus = pipeline.bus().unwrap();
        for msg in bus.iter_timed(gst::ClockTime::NONE) {
            use gst::MessageView;
            match msg.view() {
                MessageView::Error(err) => {
                    println!("Error recieved from element {:?} {}",
                             err.src().map(|s| s.path_string()),
                             err.error()
                    );
                    break;
                }
                MessageView::StateChanged(state_changed) => {
                    if state_changed
                        .src()
                        .map(|s| *s == pipeline)
                        .unwrap_or(false) {
                        println!(
                            "Pipeline state changed from {:?} to {:?}",
                            state_changed.old(),
                            state_changed.current()
                        )
                    }
                }
                MessageView::Eos(_) => break,
                _ => (),
            }
        }*/

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
                let bitrate: u32 = 500; // esempio: 500 kbps
                encoder.set_property("bitrate", bitrate);// Bitrate in kbps

                encoder.set_property_from_str("speed-preset", "ultrafast"); // Impostazione per debug
                let capsfilter_encoder = gst::ElementFactory::make("capsfilter")
                    .name("capsfilter_encoder")
                    .build()
                    .expect("Elemento 'capsfilter_encoder' non trovato");

                let caps_encoder = gst::Caps::builder("video/x-h264").build();
                capsfilter_encoder.set_property("caps", &caps_encoder);


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
                        &capsfilter_encoder,
                        &rtp_payload,
                        &udpsink,
                        &capsfilter_rtp,
                    ])?;

                    gst::Element::link_many(&[
                        &src,
                        &capsfilter_src,
                        &videoconvert,
                        &encoder,
                        &capsfilter_encoder,
                        &rtp_payload,
                        &capsfilter_rtp,
                        &udpsink,
                    ])?;

                   /* gst::debug_bin_to_dot_file_with_ts(
                        &pipeline_locked,
                        gst::DebugGraphDetails::ALL,
                        "sender-pipeline",
                    );
        */
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
