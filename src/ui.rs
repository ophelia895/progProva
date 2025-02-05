pub mod ui {

    use crate::receiver::start_video_receiver;
use egui::TextureHandle;
    use eframe::epaint::textures::TextureOptions;
    use egui::{Button, Color32, ColorImage, Context, Image, ImageButton, Key, Pos2, Rect, Rounding, Stroke};
    use egui::load::SizedTexture;
    use gstreamer::Element;
    use crate::{MouseDragHandler, MyApp, State};
    use crate::capture::capture::{get_monitors};
    use crate::State::{MainMenu, Sending};

    const TOP_PANEL_HEIGHT: f32 = 40.0;
    const SIDE_PANEL_WIDTH: f32 = 85.0;
    use gstreamer_app::{gst, AppSink, AppSinkCallbacks};
    use gstreamer::prelude::*;
    use crate::streaming::start_server;

    pub fn main_menu_ui(ctx: &Context, app: &mut MyApp) {
        egui::TopBottomPanel::top("title")
            .exact_height(TOP_PANEL_HEIGHT)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading("STREAM CONTROL");
            });

        egui::SidePanel::left("buttons")
            .exact_width(SIDE_PANEL_WIDTH)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                let visual = ui.visuals_mut();
                visual.widgets.active.weak_bg_fill = Color32::LIGHT_GREEN;
                if ui.add(Button::new("SENDER")).clicked() {
                    app.state = State::Sending;
                };
                ui.add_space(8.0);

                if ui.add(Button::new("RECEIVER")).clicked() {
                    app.state = State::Receiver;
                };

                ui.add_space(8.0);
                if ui.add(Button::new("CHANGE\nHOTKEYS")).clicked() {
                    app.state = State::KeysCustomization;
                };
            });

        video_ui(ctx, app );
    }

    pub fn sender_ui(ctx: &Context, app: &mut MyApp) {

        //handle hotkeys
        ctx.input(|i| {
            for (_, v, s) in app.keys.iter_mut() {
                if i.key_pressed(*v) {
                    *s = !*s;
                }
            }
        });

        egui::TopBottomPanel::top("title")
            .exact_height(TOP_PANEL_HEIGHT)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading("STREAMING");
            });

        egui::SidePanel::left("buttons")
            .exact_width(SIDE_PANEL_WIDTH)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.visuals_mut().widgets.active.weak_bg_fill = Color32::YELLOW;
                if ui.add(Button::new("RESIZE")).clicked() {
                    app.drag = MouseDragHandler::default();
                    app.state = State::PortionSelection;
                }
                ui.add_space(8.0);
                if ui.add_enabled(app.crop.is_some(), Button::new("FULL SIZE")).clicked() {
                    app.crop = None;
                }
                ui.add_space(8.0);
                ui.visuals_mut().widgets.active.weak_bg_fill = Color32::RED;
                if ui.add(Button::new("BACK")).clicked() {
                    //app.state = State::MonitorSelection;
                }
                ui.add_space(8.0);
                if ui.add(Button::new("MAIN MENU")).clicked() {
                    app.state = MainMenu;
                }
            });

        //start_streaming(ctx,app);
        start_server();
        video_ui(ctx, app);
    }


    pub fn connection_ui(ctx: &Context, app: &mut MyApp){



        egui::TopBottomPanel::top("title")
            .exact_height(TOP_PANEL_HEIGHT)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading("STREAMING");
            });

        egui::SidePanel::left("buttons")
            .exact_width(SIDE_PANEL_WIDTH)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.visuals_mut().widgets.active.weak_bg_fill = Color32::RED;
                if ui.add(Button::new("CLOSE")).clicked() {
                    app.ip_address=String::new();
                    app.state = State::Receiver;
                }
            });
        // Avvia la ricezione video se non è già stata avviata
        if !app.video_receiver_started {
            app.video_receiver_started = true;

            let sender_clone = app.sender_channel.clone(); // Clona il sender
            if let Err(e) = start_video_receiver(ctx.clone(), sender_clone) {
                eprintln!("Errore nell'avvio della ricezione video: {:?}", e);
            }
        }

        // Visualizza il video
        rece_ui(ctx,app);


    }
    pub fn update_video_texture(ctx: &Context, app: &mut MyApp) {
        if let Some(receiver) = &app.receiver_channel {
            while let Ok(image) = receiver.try_recv() {
                let texture = ctx.load_texture(
                    "video_frame_texture",
                    image.clone(),
                    egui::TextureOptions::LINEAR,
                );
                app.texture = Some(texture);
            }
        }
    }





    fn create_texture_from_image(ctx: &egui::Context, image: &ColorImage) -> TextureHandle {
        // I dati dei pixel sono già in formato `Color32`, quindi possiamo passarli direttamente.
        let bytes = image.pixels.iter().flat_map(|color| color.to_array()).collect::<Vec<u8>>();

        // Crea la texture direttamente con opzioni di filtro
        let options = egui::TextureOptions {
            magnification: egui::TextureFilter::Linear,  // Imposta il filtro di ingrandimento
            minification: egui::TextureFilter::Linear,   // Imposta il filtro di riduzione
            ..Default::default() // Usa le opzioni di default per gli altri campi
        };

        ctx.load_texture(
            "video_frame_texture",  // Nome della texture
            egui::ColorImage {
                size: image.size,
                pixels: image.pixels.clone(),
            },
            options,  // Passa le opzioni di texture
        )
    }

    pub fn rece_ui(ctx: &egui::Context, app: &mut MyApp) {
        // Assicurati che app.texture sia aggiornato con l'immagine ricevuta
        update_video_texture(ctx,app);

        // Visualizza il video
        video_ui(ctx, app);
    }
    /*  pub fn monitor_selection_ui(ctx: &Context, app: &mut MyApp, screenshots: Vec<ColorImage>) {
          egui::TopBottomPanel::top("title")
              .exact_height(TOP_PANEL_HEIGHT * 2.0)
              .resizable(false)
              .show(ctx, |ui| {
                  ui.add_space(8.0);
                  ui.heading("MONITOR SELECTION");
                  ui.add_space(4.0);
                  ui.heading("click the monitor you want to show")
              });

          egui::SidePanel::left("buttons")
              .exact_width(SIDE_PANEL_WIDTH * 0.8)
              .resizable(false)
              .show(ctx, |ui| {
                  ui.add_space(8.0);
                  let visual = ui.visuals_mut();
                  visual.widgets.active.weak_bg_fill = Color32::RED;
                  if ui.add(Button::new("MAIN MENU")).clicked() {
                      app.state = MainMenu;
                  }
              });

          egui::CentralPanel::default().show(ctx, |ui| {
              ui.horizontal_centered(|ui| {
                  let n: f32 = get_monitors().len() as f32;
                  let available = ui.available_size();

                  let mut cnt = 0;
                  for s in screenshots {
                      let texture = ctx.load_texture(format!("{}", cnt), s, TextureOptions::LINEAR);
                      let tex_size = texture.size();
                      let tex_size_f32 = (tex_size[0] as f32, tex_size[1] as f32);
                      let scale_x = available[0] / tex_size_f32.0;
                      let scale_y = available[1] / tex_size_f32.1;
                      let scale = scale_x.min(scale_y);
                      let scaled_size = (tex_size_f32.0 * scale / n, tex_size_f32.1 * scale / n);
                      if ui.add(ImageButton::new(SizedTexture::new(texture.id(), scaled_size))).clicked() {
                          app.monitor = get_monitors().into_iter().nth(cnt).unwrap();
                          app.state = Sending;
                      };
                      cnt += 1;
                  }
              })
          });
      }
  */
    pub fn portion_selection_ui(ctx: &Context, app: &mut MyApp) {
        egui::TopBottomPanel::top("title")
            .exact_height(TOP_PANEL_HEIGHT * 1.8)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(1.0);
                    if ui.add(Button::new("BACK")).clicked(){
                        app.state = Sending;
                    };
                    ui.add_space(2.0);
                    ui.heading("RESIZE AREA SHOWN");
                });
                ui.add_space(4.0);
                ui.heading("drag with your mouse to select the area")
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                if let Some(ref mut texture) = &mut app.texture {
                    let tex_size = texture.size();
                    let tex_size_f32 = (tex_size[0] as f32, tex_size[1] as f32);
                    let available = ui.available_size();

                    let scale_x = available[0] / tex_size_f32.0;
                    let scale_y = available[1] / tex_size_f32.1;

                    let scale = scale_x.min(scale_y);
                    let scaled_size = (tex_size_f32.0 * scale, tex_size_f32.1 * scale);

                    let mut sx1 = 0.0;
                    let mut sx2 = 0.0;
                    let mut sy1 = 0.0;
                    let mut sy2 = 0.0;
                    let mut square_renderable = false;
                    ctx.input(|i| {
                        if i.pointer.button_pressed(egui::PointerButton::Primary) {
                            app.drag.pressed = true;
                            app.drag.start_pos = i.pointer.press_origin().unwrap();
                        }
                        if app.drag.pressed == true {

                            //square highlighter of crop area handling
                            if let Some(s) = i.pointer.press_origin() {
                                if let Some(f) = i.pointer.latest_pos() {
                                    square_renderable = true;
                                    sx1 = s.x;
                                    sy1 = s.y;
                                    sx2 = f.x;
                                    sy2 = f.y;
                                }
                            }

                            if i.pointer.button_released(egui::PointerButton::Primary) {
                                app.drag.end_pos = i.pointer.latest_pos().unwrap_or(app.drag.end_pos);

                                //map x,y from mouse coord. system to image c.s.
                                let x_margin = ui.style().spacing.item_spacing.x * 1.0;
                                let y_margin = ui.style().spacing.item_spacing.y * 3.0;

                                let scale_correction = ((available[0] - scaled_size.0) / 2.0, (available[1] - scaled_size.1) / 2.0);

                                let mut x1 = (app.drag.start_pos.x - x_margin - scale_correction.0) / scale;
                                let mut y1 = (app.drag.start_pos.y - y_margin - scale_correction.1 - TOP_PANEL_HEIGHT * 1.8) / scale;
                                let mut x2 = (app.drag.end_pos.x - x_margin - scale_correction.0) / scale;
                                let mut y2 = (app.drag.end_pos.y - y_margin - scale_correction.1 - TOP_PANEL_HEIGHT * 1.8) / scale;


                                // Ensure (x1, y1) is the top-left and (x2, y2) is the bottom-right
                                if x1 > x2 {
                                    std::mem::swap(&mut x1, &mut x2);
                                }

                                if y1 > y2 {
                                    std::mem::swap(&mut y1, &mut y2);
                                }

                                //over/under size crop correction
                                if x1 < 0.0 { x1 = 0.0 };
                                if y1 < 0.0 { y1 = 0.0 };
                                if x2 > tex_size_f32.0 { x2 = tex_size_f32.0 };
                                if y2 > tex_size_f32.1 { y2 = tex_size_f32.1 };

                                //to handle crop too small
                                if x2 - x1 > 5.0 && y2 - y1 > 5.0 {
                                    app.crop = Some(Rect::from_min_max(Pos2::new(x1, y1), Pos2::new(x2, y2)));
                                }
                                app.state = Sending;
                            }
                        }
                    });

                    ui.add_sized(scaled_size, Image::from_texture(
                        SizedTexture::new(texture.id(), scaled_size)));

                    //square highlighter of crop area handling
                    if app.drag.pressed {
                        if sx1 > sx2 {
                            std::mem::swap(&mut sx1, &mut sx2);
                        }
                        if sy1 > sy2 {
                            std::mem::swap(&mut sy1, &mut sy2);
                        }
                        if sx2 - sx1 > 8.0 && sy2 - sy1 > 8.0 {
                            let rect = Rect::from_points(&[Pos2::new(sx1,sy1), Pos2::new(sx2,sy2)]);
                            ui.put(rect, Button::new("")
                                .rounding(Rounding::ZERO)
                                .stroke(Stroke::new(2.0, Color32::DARK_GRAY))
                                .fill(Color32::from_rgba_unmultiplied(160, 160, 160, 20)));
                        }
                    }
                }
            });
        });
    }

    pub fn key_customization_ui(ctx: &Context, app: &mut MyApp) {

        egui::TopBottomPanel::top("top_panel")
            .exact_height(TOP_PANEL_HEIGHT)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(1.0);
                    if ui.add(Button::new("BACK")).clicked() {
                        app.state = MainMenu;
                    };
                    ui.add_space(2.0);
                    ui.heading("CUSTOMIZE YOUR HOTKEYS");
                });

            });

        egui::CentralPanel::default().show(ctx, |ui| {
            match app.changing_keys.clone() {
                None => {
                    app.keys.sort_by(|(k1, _, _), (k2, _, _)| {k1.len().cmp(&k2.len())});
                    let mut max_width = None;
                    for (k, v, _) in app.keys.clone() {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let kc = k.clone();
                            let vc = v.clone();
                            if ui.add(Button::new("Change")).clicked() {
                                app.changing_keys = Some((k.clone(), v.clone()));
                            }
                            ui.add_space(2.0);
                            let w =
                                if max_width.is_none() {
                                    max_width = Some(ui.heading(kc).rect.width());
                                    max_width.unwrap()
                                }
                                else {
                                    ui.heading(kc).rect.width()
                                };
                            ui.add_space(max_width.unwrap() * 3.0 - w);
                            ui.heading(format!("{:?}", vc).to_uppercase());
                        });
                    }
                }

                Some((k, _)) => {
                    ui.centered_and_justified(|ui| {
                        ui.heading("Press any key...");
                    });
                    ctx.input(|i| {
                        for key in Key::ALL {
                            if i.key_pressed(key.clone()) {
                                app.keys.iter_mut().find(|(kk,_, _)| {kk == &k}).unwrap().1 = key.clone();
                                app.changing_keys = None;
                                break;
                            }
                        }
                    });
                }
            }
        });

    }


    pub fn receiver_ui(ctx: &Context, app: &mut MyApp) {
        egui::TopBottomPanel::top("title")
            .exact_height(TOP_PANEL_HEIGHT * 2.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading("ESTABLISH CONNECTION");
                ui.add_space(4.0);
                ui.heading("Specify the address of the caster it should connect to.");
            });

        egui::SidePanel::left("buttons")
            .exact_width(SIDE_PANEL_WIDTH)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.visuals_mut().widgets.active.weak_bg_fill = Color32::YELLOW;
                ui.add_space(8.0);
                if ui.add(Button::new("MAIN MENU")).clicked() {
                    app.state = MainMenu;
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(16.0);
                ui.label("Enter IP Address:");

                let mut parts = app.ip_address.split('.').map(|s| s.to_string()).collect::<Vec<String>>();
                while parts.len() < 4 {
                    parts.push(String::new());
                }

                for (i, part) in parts.iter_mut().enumerate() {
                    let response = ui.add(
                        egui::TextEdit::singleline(part)
                            .hint_text("0-255")
                            .desired_width(30.0),
                    );

                    if response.changed() {
                        *part = part.chars().filter(|c| c.is_digit(10)).take(3).collect();
                        if let Ok(num) = part.parse::<u8>() {
                            if num > 255 {
                                *part = "255".to_string();
                            }
                        }
                    }

                    if i < 3 {
                        ui.label(".");
                    }
                }

                app.ip_address = parts.join(".");

                if ui.add(Button::new("Connect")).clicked() {
                    println!("Connecting to IP: {}", app.ip_address);

                    // Avvia il ricevitore video
                    if !app.video_receiver_started {
                        app.video_receiver_started = true;

                        let sender_clone = app.sender_channel.clone();
                        if let Err(e) = start_video_receiver(ctx.clone(), sender_clone) {
                            eprintln!("Errore nell'avvio della ricezione video: {:?}", e);
                        }
                    }

                    app.state = State::Connection; // Passa allo stato di ricezione del video
                }
            });
        });

        // Assicurati che il video venga aggiornato
        update_video_texture(ctx, app);

    }

    pub fn video_ui(ctx: &Context, app: &mut MyApp) {
        update_video_texture(ctx, app); // Aggiorna la texture prima di disegnarla

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                if let Some(texture) = &app.texture {
                    let tex_size = texture.size();
                    let available = ui.available_size();
                    let scale_x = available[0] / tex_size[0] as f32;
                    let scale_y = available[1] / tex_size[1] as f32;
                    let scale = scale_x.min(scale_y);
                    let scaled_size = (tex_size[0] as f32 * scale, tex_size[1] as f32 * scale);

                    ui.add_sized(scaled_size, Image::from_texture(
                        SizedTexture::new(texture.id(), scaled_size)));
                } else {
                    ui.label("Nessun video ricevuto...");
                }
            });
        });
    }




}

