use std::error::Error;
use std::sync::mpsc;
use std::thread;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer::glib;
use egui::ColorImage;
use eframe::epaint::Color32;

pub fn start_receiver(tx: mpsc::Sender<ColorImage>, ipaddr: String) -> Result<(), Box<dyn Error>> {
    gst::init()?; // Inizializzazione GStreamer
    let server_address = format!("{}:50496", ipaddr);
    println!("In ascolto su {}", server_address);

    let pipeline = gst::Pipeline::new(None);

    // Crea gli elementi della pipeline
    let src = gst::ElementFactory::make("udpsrc").name("src").build()?;
    src.set_property("address", &ipaddr);
    src.set_property("port", &50496i32);

    let capsfilter_rtp = gst::ElementFactory::make("capsfilter").name("capsfilter_rtp").build()?;
    let caps_rtp = gst::Caps::builder("application/x-rtp")
        .field("media", &"video")
        .field("encoding-name", &"H264")
        .field("payload", &96)
        .build();
    capsfilter_rtp.set_property("caps", &caps_rtp);

    let rtp_depay = gst::ElementFactory::make("rtph264depay").build()?;
    let decoder = gst::ElementFactory::make("avdec_h264").build()?;
    let videoconvert = gst::ElementFactory::make("videoconvert").build()?;
    let appsink = gst::ElementFactory::make("appsink").name("appsink").build()?;
    appsink.set_property("emit-signals", &true);
    appsink.set_property("sync", &true);

    // Aggiungi gli elementi alla pipeline
    pipeline.add_many(&[&src, &capsfilter_rtp, &rtp_depay, &decoder, &videoconvert, &appsink])?;
    gst::Element::link_many(&[&capsfilter_rtp, &rtp_depay, &decoder, &videoconvert, &appsink])?;

    // Collega manualmente src a capsfilter_rtp
    src.link(&capsfilter_rtp)?;

    gst::debug_bin_to_dot_file_with_ts(&pipeline, gst::DebugGraphDetails::ALL, "receiver-pipeline");

    // Gestione del segnale "new-sample" di appsink
    let appsink_clone = appsink.clone();
    appsink.connect("new-sample", false, move |_| {
        if let Some(sample) = appsink_clone.property::<Option<gst::Sample>>("last-sample") {
            if let Some(buffer_ref) = sample.buffer() {
                if let Ok(image) = decode_buffer_to_color_image(buffer_ref) {
                    if let Err(e) = tx.send(image) {
                        eprintln!("Errore nell'invio dell'immagine: {}", e);
                    }
                } else {
                    eprintln!("Errore nella decodifica del buffer.");
                }
            } else {
                eprintln!("Errore: buffer non trovato nel sample.");
            }
        } else {
            eprintln!("Errore: sample non trovato.");
        }
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

fn decode_buffer_to_color_image(buffer: &gst::BufferRef) -> Result<ColorImage, Box<dyn Error>> {
    let map = buffer.map_readable()?;
    let data = map.as_slice();

    let width = 640;
    let height = 480;

    if data.len() < width * height * 3 {
        return Err("Dati insufficienti nel buffer".into());
    }

    let pixels = (0..width * height)
        .map(|i| {
            let r = data[i * 3] as u8;
            let g = data[i * 3 + 1] as u8;
            let b = data[i * 3 + 2] as u8;
            Color32::from_rgb(r, g, b)
        })
        .collect();

    Ok(ColorImage {
        size: [width as usize, height as usize],
        pixels,
    })
}
