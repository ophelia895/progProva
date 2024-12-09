mod capture;
mod ui;
mod streaming;
mod receiver;

use crate::capture::capture::{capture, crop_color_image, get_monitors, image_from_path, primary_monitor};
use eframe::egui::{self, ColorImage, TextureHandle};
use eframe::{App, Frame, HardwareAcceleration};
use egui::{Key, Rect, ViewportBuilder, Visuals};
use egui::{TextureOptions, Vec2};
use std::cmp::PartialEq;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use eframe::emath::Pos2;
use gstreamer::Context;
use xcap::Monitor;
use crate::receiver::start_receiver;
use crate::State::{MainMenu, MonitorSelection, PortionSelection, Receiver, Sending, KeysCustomization};

use crate::ui::ui::*;

const FRAMERATE: usize = 60;
const FRAMEPERIOD: f64 = 1.0 / (FRAMERATE as f64);
const WAIT_FRAME: Duration = Duration::from_micros((FRAMEPERIOD * 1_000_000.0) as u64);
const WINDOW_NAME: &str = "Screen Caster";

#[derive(PartialEq, PartialOrd, Debug)]
enum State
{
    MainMenu,
    MonitorSelection,
    Sending,
    Receiver,
    Connetion,
    PortionSelection,
    KeysCustomization,
}

struct MouseDragHandler {
    start_pos: Pos2,
    end_pos: Pos2,
    pressed: bool,
}

impl Default for MouseDragHandler {
    fn default() -> Self {
        MouseDragHandler {
            start_pos: Pos2::default(),
            end_pos: Pos2::default(),
            pressed: false,
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    //configure the native window options
    let vpb = ViewportBuilder {
        title: Some(WINDOW_NAME.to_string()),
        app_id: Some("Window id".to_string()),
        position: None,
        inner_size: None,
        min_inner_size: Some(Vec2::new(450.0, 200.0)),
        max_inner_size: None,
        clamp_size_to_monitor_size: Some(true),
        fullscreen: None,
        maximized: None,
        resizable: None,
        transparent: None,
        decorations: None,
        icon: None,
        active: Some(true),
        visible: Some(true),
        fullsize_content_view: None,
        title_shown: Some(true),
        titlebar_buttons_shown: None,
        titlebar_shown: None,
        drag_and_drop: Some(true),
        taskbar: None,
        close_button: None,
        minimize_button: None,
        maximize_button: None,
        window_level: None,
        mouse_passthrough: None,
        window_type: None,
    };
    let options = eframe::NativeOptions {
        viewport: vpb,
        vsync: false,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        hardware_acceleration: HardwareAcceleration::Required,
        renderer: Default::default(),
        run_and_return: false,
        event_loop_builder: None,
        window_builder: None,
        shader_version: None,
        centered: false,
        persist_window: false,
        persistence_path: None,
        dithering: false,
    };

    // Run the app
    eframe::run_native(
        "Screen Caster App",    // Window title
        options,
        Box::new(|_cc| {
            Ok(Box::new(MyApp::new(primary_monitor().unwrap())))
        }),
    )
}
struct MyApp {
    texture: Option<TextureHandle>, // To store the image texture
    receiver_channel: Option<mpsc::Receiver<ColorImage>>, // Canale per ricevere immagini
    timer: Instant,
    state: State,
    monitor: Monitor,
    main_menu_img: Option<ColorImage>,
    drag: MouseDragHandler,
    monitor_preview: Option<Vec<ColorImage>>,
    crop: Option<Rect>,
    keys: Vec<(String, Key, bool)>,
    changing_keys: Option<(String, Key)>,
    ip_address: String,

}

impl MyApp {
    fn new(monitor: Monitor) -> Self {

        let mut keys = Vec::new();
        keys.push(("PAUSE".to_string(), Key::Space, false));
        keys.push(("HIDE".to_string(), Key::H, false));
        keys.push(("TERMINATE".to_string(), Key::Escape, false));
        let main_menu_img = image_from_path("assets/no_signal.jpg");

        MyApp {
            texture: None,
            receiver_channel: None,
            timer: Instant::now(),
            state: MainMenu,
            monitor,
            main_menu_img,
            drag: MouseDragHandler::default(),
            monitor_preview: None,
            crop: None,
            keys,
            changing_keys: None,
            ip_address:  String::new()
        }
    }

    // Funzione per avviare il thread di ricezione video
    fn start_video_receiver(&mut self, ctx: & egui::Context) {
        let (tx, rx) = mpsc::channel::<ColorImage>(); // Canale per inviare le immagini

        //il receiver sarà uno cioè l'ui
        self.receiver_channel = Some(rx);
        //avrò più sender quanti sono i client che vogliono connettersi al server di streaming
        //ogni client manderà un'immagine al canale per l'ui
        let txc=tx.clone();
        //let colorImage = Arc::new(Mutex::new(self));
        //
        // let app = colorImage.clone();
        // Cloniamo MyApp e lo condividiamo con il thread
        // Avvia un thread di ricezione video
        let  ipaddr= self.ip_address.clone();
        thread::spawn(move||{
            // Otteniamo una copia di Arc<Mutex<MyApp>>
            // let  appl = app.lock().unwrap();
            // Blocchiamo il Mutex per ottenere un riferimento sicuro
            if let Err(e) = receiver::start_receiver(txc, ipaddr) {
                eprintln!("Errore durante la ricezione del video: {}", e);
            }
        });


        // Crea la texture per visualizzare l'immagine ricevuta
        self.texture = Some(ctx.load_texture(
            "video_frame",
            ColorImage::new([1, 1], Default::default()), // Immagine iniziale vuota
            Default::default(),
        ));
    }

    // Funzione per aggiornare l'interfaccia utente con il video ricevuto
    pub fn update_video_texture(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.receiver_channel {
            if let Ok(image) = receiver.try_recv() {
                // Se ricevi una nuova immagine dal canale
                if let Some(texture_handle) = self.texture.as_mut() {
                    // Usa un riferimento mutabile per la texture
                    texture_handle.set(image, TextureOptions::LINEAR);
                }
            }
        }
    }
}


impl App for MyApp {

    //application main loop
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {

        //set theme for the window
        //should be set outside the update function, but it does not work there :)
        ctx.set_visuals(Visuals::dark());

        //load main menu image
        if self.state == MainMenu
            || self.keys.iter().find(|(k,_,_)| {k == "HIDE"}).unwrap().2 {
            self.texture = Some(ctx.load_texture("image_texture", self.main_menu_img.as_ref().unwrap().clone(), TextureOptions::LINEAR));
        }

        if self.state != MonitorSelection {
            self.monitor_preview = None;
        }

        //if terminate key pressed return to main menu
        if self.keys.iter().find(|(k,_,_)| {k == "TERMINATE"}).unwrap().2 {
            if self.state == MainMenu {
                //restore terminate key default value
                if let Some((_, _, ref mut terminate)) = self.keys.iter_mut().find(|(k, _, _)| k == "TERMINATE") {
                    *terminate = false;
                }

            }
            else {
                self.state = MainMenu;
            }
        }

        //ui render based on the app state
        match self.state {
            MainMenu => {
                main_menu_ui(ctx, self);
            }
            Sending => {
                sender_ui(ctx, self);
            }
            Receiver=>{
                receiver_ui(ctx,self);
            }
            State::Connetion=>{

                connetion_ui(ctx,self);

            }
            MonitorSelection => {

                let mut screenshots;
                match &self.monitor_preview {
                    None => {
                        screenshots = Vec::new();
                        for m in get_monitors() {
                            if let Ok(preview) = capture(&m) {
                                screenshots.push(ColorImage::from_rgba_unmultiplied(
                                    [preview.width() as usize, preview.height() as usize], preview.as_raw()));
                            }
                        }
                        self.monitor_preview = Some(screenshots.clone());
                    }
                    Some(v) => {
                        screenshots = v.clone();
                    }
                }
                monitor_selection_ui(ctx, self, screenshots);
            }


            PortionSelection => {
                portion_selection_ui(ctx, self);
            }
            KeysCustomization => {
                key_customization_ui(ctx, self);
            }
            _ => {}
        }

        //capture new frame and set it as a texture
        //after page render to avoid slowing it down
        if self.state == Sending
            && !self.keys.iter().find(|(k, _, _)| k == "PAUSE").unwrap().2
            && self.timer.elapsed() >= WAIT_FRAME
            && !self.keys.iter().find(|(k,_,_)| {k == "HIDE"}).unwrap().2

        {

            if let Ok(img) = capture(&self.monitor) {
                let mut color_img = ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
                if let Some(rect) = self.crop {
                    color_img = crop_color_image(&color_img, rect.min.x as u32, rect.min.y as u32, rect.width() as u32, rect.height() as u32);
                }
                self.texture = Some(ctx.load_texture("image_texture", color_img, TextureOptions::LINEAR));
                //reset timer that control framerate
                self.timer = Instant::now();
            }
        }

        //this is used to call update only when a new  frame is needed
        let t = self.timer.elapsed();
        ctx.request_repaint_after(
            if WAIT_FRAME > t {
                WAIT_FRAME - t
            } else {
                Duration::from_secs(0)
            }
        );
    }
}
