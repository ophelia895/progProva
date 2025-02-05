use gstreamer::prelude::*;
use gstreamer_app::{AppSink, AppSinkCallbacks};
use gstreamer_video::{VideoFrame, VideoInfo};
use eframe::egui::{ColorImage, Color32};
use std::sync::mpsc;
use gstreamer as gst;
use gstreamer_video as gst_video;

pub fn start_video_receiver(ctx: egui::Context, sender: mpsc::Sender<ColorImage>) -> Result<(), Box<dyn std::error::Error>> {
    // Inizializza GStreamer
    gst::init()?;

    // Utilizza una pipeline ottimizzata con meno buffering
  /*  let pipeline = gst::parse_launch(
        "udpsrc port=5000 caps=\"application/x-rtp,media=video,encoding-name=H264,payload=96\" \
        ! rtph264depay ! decodebin ! videoconvert ! videoscale ! video/x-raw,format=RGB,width=640,height=360 \
        ! appsink name=videosink"
    )?;
*/
    let ip="192.168.216.246";
    // Crea la pipeline utilizzando il parametro ip per il bind della sorgente UDP
    let pipeline_str = format!(
        "udpsrc address={} port=5000 caps=\"application/x-rtp,media=video,encoding-name=H264,payload=96\" \
         ! rtph264depay ! decodebin ! videoconvert ! videoscale ! video/x-raw,format=RGB,width=640,height=360 \
         ! appsink name=videosink",
        ip
    );
    let pipeline = gst::parse_launch(&pipeline_str)?;

    let pipeline = pipeline
        .downcast::<gst::Pipeline>()
        .map_err(|_| "Failed to downcast pipeline to gst::Pipeline")?;

    let appsink = pipeline.by_name("videosink")
        .ok_or("Cannot find appsink element")?
        .downcast::<AppSink>()
        .map_err(|_| "Cannot cast element to AppSink")?;

    let sender_clone = sender.clone();

    appsink.set_callbacks(
        AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                // Preleva il sample
                let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                let buffer_ref = sample.buffer().ok_or(gst::FlowError::Error)?;

                // Ottieni i caps dal sample e crea il VideoInfo
                let caps = sample.caps().ok_or(gst::FlowError::Error)?;
                let info = VideoInfo::from_caps(&caps).map_err(|_| gst::FlowError::Error)?;

                // Copia il buffer per ottenere un buffer "owned"
                let owned_buffer = buffer_ref.copy();
                let video_frame = VideoFrame::from_buffer_readable(owned_buffer, &info)
                    .map_err(|_| gst::FlowError::Error)?;

                let width = info.width() as usize;
                let height = info.height() as usize;

                // Determina i byte per pixel in base al formato:
                let format = info.format();
                let bytes_per_pixel = match format {
                    gst_video::VideoFormat::Rgb => 3,
                    gst_video::VideoFormat::Rgb => 4,
                    _ => 3,
                };

                // Ottieni i dati del primo piano e lo stride
                let plane = video_frame.plane_data(0).map_err(|_| gst::FlowError::Error)?;
                let stride = video_frame.plane_stride()[0] as usize;

                // Prepara il vettore di pixel: ogni pixel sarà un Color32
                let mut pixels = Vec::with_capacity(width * height);

                // Itera su ogni riga
                for row in 0..height {
                    let start = row * stride;
                    // Leggi esattamente width * bytes_per_pixel byte per la riga
                    let row_slice = &plane[start..start + width * bytes_per_pixel];
                    if bytes_per_pixel == 3 {
                        // Se il formato è RGB, usa i 3 byte nell'ordine (R, G, B)
                        for chunk in row_slice.chunks_exact(3) {
                            pixels.push(Color32::from_rgb(chunk[0], chunk[1], chunk[2]));
                        }
                    } else if bytes_per_pixel == 4 {
                        // Se il formato è RGBA, ignora il canale alpha
                        for chunk in row_slice.chunks_exact(4) {
                            pixels.push(Color32::from_rgb(chunk[0], chunk[1], chunk[2]));
                        }
                    }
                }

                // Crea il ColorImage
                let image = ColorImage {
                    size: [width, height],
                    pixels,
                };

                // Invia l'immagine al thread principale e richiedi il repaint dell'UI
                if sender_clone.send(image).is_ok() {
                    ctx.request_repaint();
                }

                Ok(gst::FlowSuccess::Ok)
            })
            .build()
    );

    pipeline.set_state(gst::State::Playing)?;

    // La pipeline continua a girare in background
    Ok(())
}
