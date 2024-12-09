use std::error::Error;
use std::net::UdpSocket;
use std::sync::mpsc;
use std::{thread, time::Duration};
use eframe::epaint::Color32;
use gstreamer as gst;
use gstreamer::prelude::*;
use egui::ColorImage;
use gstreamer::glib;
use crate::MyApp;

pub fn start_receiver(tx: mpsc::Sender<ColorImage>, ipaddr: String) -> Result<(), Box<dyn Error>> {
    // Inizializzazione di GStreamer
    gst::init()?;

    // Creazione dell'indirizzo completo del server
    let server_address = format!("{}:50496", ipaddr);

    // Creazione del socket UDP
    let socket = UdpSocket::bind(&server_address)?;
    println!("Receiver UDP in ascolto su {}", server_address);

    // Creazione della pipeline GStreamer
    let pipeline = gst::Pipeline::new(None);

    // Creazione dell'elemento sorgente UDP
    let src = gst::ElementFactory::make("udpsrc")
        .name("src")
        .build()
        .expect("Elemento 'udpsrc' non trovato");

    // Imposta la porta (tipo i32)
    src.set_property("port", &50496i32);
    println!("Porta di udpsrc: {:?}", src.property::<i32>("port"));

    // Creazione degli elementi per il processing del flusso video
    let rtp_depay = gst::ElementFactory::make("rtph264depay")
        .build()
        .expect("Elemento 'rtph264depay' non trovato");

    let decoder = gst::ElementFactory::make("avdec_h264")
        .build()
        .expect("Elemento 'avdec_h264' non trovato");

    let appsink = gst::ElementFactory::make("appsink")
        .name("appsink")
        .build()
        .expect("Elemento 'appsink' non trovato");

    // Impostazione delle proprietà di appsink
    appsink.set_property("emit-signals", &true);
    appsink.set_property("sync", &false);

    // Aggiunta degli elementi alla pipeline
    pipeline.add_many(&[&src, &rtp_depay, &decoder, &appsink])?;

    // Collegamento degli elementi nella pipeline
    if let Err(err) = gst::Element::link_many(&[&src, &rtp_depay, &decoder, &appsink]) {
        eprintln!("Errore nel collegare gli elementi: {}", err);
        return Err("Collegamento fallito".into());
    }

    // Connessione al segnale "new-sample" di appsink
    let appsink_clone = appsink.clone();
    appsink.connect("new-sample", false, move |_| {
        // Estrai il sample da appsink
        if let Some(sample) = appsink_clone.property::<Option<gst::Sample>>("last-sample") {
            // Otteniamo il buffer dal sample
            if let Some(buffer_ref) = sample.buffer() {
                // Decodifica del buffer in ColorImage
                if let Ok(image) = decode_buffer_to_color_image(&buffer_ref) {
                    // Invia l'immagine al canale per il thread UI
                    if let Err(e) = tx.send(image) {
                        eprintln!("Errore nell'invio dell'immagine al canale: {}", e);
                    }
                }
            }
        }

        // Restituisci gst::FlowReturn::Ok per indicare che il frame è stato gestito
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

    // Loop per ricevere pacchetti UDP
    let mut buf = [0; 2048];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((bytes_received, src_addr)) => {
                println!("Ricevuti {} byte da {}", bytes_received, src_addr);
                // Processa i dati ricevuti (se necessario).
                // In questo caso, sono passati direttamente a GStreamer tramite udpsrc.
            }
            Err(e) => {
                eprintln!("Errore nella ricezione del pacchetto: {}", e);
            }
        }
    }

    // Arresto della pipeline
    pipeline.set_state(gst::State::Null)?;

    Ok(())
}

// Funzione per decodificare un buffer H264 in un'immagine ColorImage
fn decode_buffer_to_color_image(buffer: &gst::BufferRef) -> Result<ColorImage, Box<dyn Error>> {
    let size = buffer.size();
    let binding = buffer.map_readable()?;
    let data = binding.as_slice();

    let width = 640; // Larghezza simulata
    let height = 480; // Altezza simulata
    let pixels = vec![Color32::BLACK; (width * height) as usize];

    Ok(ColorImage {
        size: [width as usize, height as usize],
        pixels,
    })
}
