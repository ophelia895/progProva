use std::error::Error;
use std::sync::mpsc;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer::glib;
use egui::ColorImage;
use eframe::epaint::Color32;

use crate::MyApp;

pub fn start_receiver(tx: mpsc::Sender<ColorImage>, ipaddr: String) -> Result<(), Box<dyn Error>> {
    // Inizializzazione di GStreamer
    gst::init()?;

    // Creazione dell'indirizzo completo del server
    let server_address = format!("{}:50496", ipaddr);
    println!("In ascolto su {}", server_address);

    // Creazione della pipeline GStreamer
    let pipeline = gst::Pipeline::new(None);

    // Elementi della pipeline
    let src = gst::ElementFactory::make("udpsrc")
        .name("src")
        .build()
        .expect("Elemento 'udpsrc' non trovato");
    src.set_property("port", &50496i32);

    let rtp_depay = gst::ElementFactory::make("rtph264depay")
        .build()
        .expect("Elemento 'rtph264depay' non trovato");

    let capsfilter_rtp = gst::ElementFactory::make("capsfilter")
        .name("capsfilter_rtp")
        .build()
        .expect("Elemento 'capsfilter_rtp' non trovato");

    let caps_rtp = gst::Caps::builder("application/x-rtp")
        .field("media", &"video")
        .field("encoding-name", &"H264")
        .field("payload", &96) // Deve corrispondere al payload type del sender
        .build();
    capsfilter_rtp.set_property("caps", &caps_rtp);

    let decoder = gst::ElementFactory::make("avdec_h264")
        .build()
        .expect("Elemento 'avdec_h264' non trovato");

    let videoconvert = gst::ElementFactory::make("videoconvert")
        .build()
        .expect("Elemento 'videoconvert' non trovato");

    let appsink = gst::ElementFactory::make("appsink")
        .name("appsink")
        .build()
        .expect("Elemento 'appsink' non trovato");
    appsink.set_property("emit-signals", &true);
    appsink.set_property("sync", &false);

    // Aggiunta degli elementi alla pipeline
    pipeline.add_many(&[&src, &capsfilter_rtp,&rtp_depay, &decoder, &videoconvert, &appsink])?;

    // Collegamento degli elementi
    gst::Element::link_many(&[&src,&capsfilter_rtp, &rtp_depay, &decoder, &videoconvert, &appsink])?;

    gst::debug_bin_to_dot_file_with_ts(
        &pipeline,
        gst::DebugGraphDetails::ALL,
        "receiver-pipeline",
    );

    // Gestione del segnale "new-sample" di appsink
    let appsink_clone = appsink.clone();
    appsink.connect("new-sample", false, move |_| {
        // Estrazione del sample
        if let Some(sample) = appsink_clone.property::<Option<gst::Sample>>("last-sample") {
            if let Some(buffer_ref) = sample.buffer() {
                // Decodifica del buffer in immagine
                if let Ok(image) = decode_buffer_to_color_image(buffer_ref) {
                    if let Err(e) = tx.send(image) {
                        eprintln!("Errore nell'invio dell'immagine: {}", e);
                    }
                }
            }
        }
        // Restituisci gst::FlowReturn::Ok
        Some(glib::Value::from(gst::FlowReturn::Ok))
    });




    // Avvio della pipeline
    match pipeline.set_state(gst::State::Playing) {
        Ok(gst::StateChangeSuccess::Success) => println!("Pipeline avviata con successo."),
        Ok(_) => println!("Pipeline in uno stato non definitivo."),
        Err(err) => {
            eprintln!("Errore nell'avvio della pipeline: {}", err);
            return Err("Errore nel cambio di stato".into());
        }
    }

    // Mantieni la pipeline attiva
    let bus = pipeline.bus().unwrap();
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;
        match msg.view() {
            MessageView::Eos(..) => {
                println!("Fine dello streaming.");
                break;
            }
            MessageView::Error(err) => {
                eprintln!("Errore nella pipeline: {}", err.error());
                break;
            }
            _ => (),
        }
    }

    // Arresto della pipeline
    pipeline.set_state(gst::State::Null)?;

    Ok(())
}

// Funzione per decodificare un buffer in un'immagine ColorImage
fn decode_buffer_to_color_image(buffer: &gst::BufferRef) -> Result<ColorImage, Box<dyn Error>> {
    let size = buffer.size();
    let data = buffer.map_readable()?.as_slice();

    // Esempio simulato: restituisce un'immagine nera
    let width = 640; // Simulazione: modifica in base ai tuoi dati
    let height = 480;
    let pixels = vec![Color32::BLACK; (width * height) as usize];

    Ok(ColorImage {
        size: [width as usize, height as usize],
        pixels,
    })
}
