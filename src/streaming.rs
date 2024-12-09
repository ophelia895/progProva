pub mod streaming {
    use gstreamer as gst;
    use gstreamer::prelude::*;
    use std::error::Error;
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use egui::Context;
    use crate::MyApp;

    fn handle_connection(stream: TcpStream, pipeline: Arc<Mutex<gst::Pipeline>>) -> Result<(), Box<dyn Error>> {
        let receiver_ip = stream.peer_addr()?.ip().to_string();
        println!("Nuova connessione da: {}", receiver_ip);

        {
            let pipeline_locked = pipeline.lock().unwrap();
            if let Some(sink) = pipeline_locked.by_name("udpsink") {
                // Pausa temporanea della pipeline
                pipeline_locked.set_state(gst::State::Paused)?;

                // Aggiorna l'indirizzo del sink
                sink.set_property("host", &receiver_ip);

                // Riprendi la pipeline
                pipeline_locked.set_state(gst::State::Playing)?;
            }
        }
        Ok(())
    }

    pub fn start_streaming(ctx: &Context, app: &mut MyApp) -> Result<(), Box<dyn Error>> {
        gst::init()?; // Inizializzazione di GStreamer

        let pipeline = gst::Pipeline::new(None);
        let shared_pipeline = Arc::new(Mutex::new(pipeline));

        // Creazione degli elementi della pipeline
        let src = gst::ElementFactory::make("videotestsrc")
            .name("src")
            .build()
            .expect("Elemento 'videotestsrc' non trovato");

        // Cambia direttamente a un formato che sia compatibile con il resto della pipeline
        let capsfilter_src = gst::ElementFactory::make("capsfilter")
            .name("capsfilter_src")
            .build()
            .expect("Elemento 'capsfilter_src' non trovato");
        let caps_src = gst::Caps::builder("video/x-raw")
            .field("format", &"I420_10LE") // Cambia a I420, che è supportato da x264enc
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
            .field("format", &"I420") // Assicurati di usare lo stesso formato che x264enc supporta
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
        udpsink.set_property("host", &"127.0.0.1");
        udpsink.set_property("port", &50496i32);

        let autovideosink = gst::ElementFactory::make("autovideosink")
            .name("autovideosink")
            .build()
            .expect("Elemento 'autovideosink' non trovato");

        // Aggiungi un tee per dividere il flusso
        let tee = gst::ElementFactory::make("tee")
            .name("tee")
            .build()
            .expect("Elemento 'tee' non trovato");

        // Aggiungi gli elementi alla pipeline
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
               // &autovideosink,
            ])?;

            // Collega gli elementi
            gst::Element::link_many(&[
                &src,
                &capsfilter_src,
                &videoconvert,
                &capsfilter_convert,
                &encoder,
                &rtp_payload,
                &tee,
            ])?;

            // Collega il tee ai due rami (udpsink e autovideosink)
            let tee_src_pad_udpsink = tee
                .request_pad_simple("src_%u")
                .expect("Impossibile richiedere un pad al tee");
            let udpsink_sink_pad = udpsink
                .static_pad("sink")
                .expect("Pad sink non trovato in udpsink");
            tee_src_pad_udpsink.link(&udpsink_sink_pad)?;

            let tee_src_pad_videosink = tee
                .request_pad_simple("src_%u")
                .expect("Impossibile richiedere un pad al tee");
           /* let autovideosink_sink_pad = autovideosink
                .static_pad("sink")
                .expect("Pad sink non trovato in autovideosink");
            tee_src_pad_videosink.link(&autovideosink_sink_pad)?;*/
        }

        // Avvio della pipeline
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

        // Gestione del bus con log dettagliati
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
                        // Log degli errori con maggiore dettaglio
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
