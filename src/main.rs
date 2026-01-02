//use std::io::{Write};
use eframe::egui::{self, TopBottomPanel, MenuBar, RichText};

//-- xpacket --
const XPACKET_BEGIN: &[u8] = b"<?xpacket begin=";
const XPACKET_END_START: &[u8] = b"<?xpacket end=";
const XPACKET_END: &[u8] = b"?>";

struct XpacketInfo {
    data: Vec<u8>,
    offset: usize,
    size: usize,
}

fn extract_xpacket(data: &[u8]) -> Result<XpacketInfo, String> {
    let begin = memchr::memmem::find(data, XPACKET_BEGIN).ok_or("xpacket beginning marker not found!".to_string())?;
    let end_start = memchr::memmem::find(&data[begin..], XPACKET_END_START).ok_or("xpacket end marker not found!".to_string())? + begin;
    let end = memchr::memmem::find(&data[end_start..], XPACKET_END).ok_or("xpacket end not found!".to_string())? + end_start + XPACKET_END.len();

    Ok(XpacketInfo {data: data[begin..end].to_vec(), offset: begin, size: end - begin})
}
//-- end xpacket -- 

//-- xml --
#[derive(Debug)]
struct XmlNode {
    name: String,
    attributes: Vec<(String, String)>,
    children: Vec<XmlNode>,
    text: Option<String>,
}

fn build_tree(node: roxmltree::Node) -> Option<XmlNode> {
    if !node.is_element() {
        return None;
    }

    let name = node.tag_name().name().to_string();

    let attributes = node
        .attributes()
        .map(|a| (a.name().to_string(), a.value().to_string()))
        .collect();

    let mut children = Vec::new();
    let mut text = None;

    for c in node.children() {
        if let Some(child) = build_tree(c) {
            children.push(child);
        } else if c.is_text() {
            let t = c.text().unwrap().trim();
            if !t.is_empty() {
                text = Some(t.to_string());
            }
        }
    }

    Some(XmlNode {name, attributes, children, text})
}
//-- end xml --

//-- GUI --
struct XmpeekApp {
    root: Option<XmlNode>,
    error_message: Option<String>,
    current_file: Option<String>,
    file_to_load: Option<String>,

    xpacket_data: Option<Vec<u8>>,
    xpacket_offset: Option<usize>,
    xpacket_size: Option<usize>,
}

impl XmpeekApp {
    fn load_file(&mut self, path: &str) {
        match std::fs::read(path) {
            Ok(data) => match extract_xpacket(&data) {
                Ok(info) => {
                    self.xpacket_data = Some(info.data.clone());
                    self.xpacket_offset = Some(info.offset);
                    self.xpacket_size = Some(info.size);
                    
                    let xml = String::from_utf8_lossy(&info.data);
                    match roxmltree::Document::parse(&xml) {
                        Ok(doc) => {
                            self.root = Some(build_tree(doc.root_element()).unwrap());
                            self.current_file = Some(path.to_string());
                        }
                        Err(e) => self.error_message = Some(format!("Failed to parse XML: {}", e)),
                    }
                }
                Err(e) => self.error_message = Some(format!("Failed to extract xpacket: {}", e)),
            },
            Err(e) => self.error_message = Some(format!("Failed to read file: {}", e)),
        }
    }
}

impl eframe::App for XmpeekApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        //open a file
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            self.load_file(path.to_str().unwrap());
                        }
                    }
                    if ui.button("Save xpacket").clicked() {
                        //export the xpacket as a file
                        if let Some(data) = &self.xpacket_data {
                            if let Some(path) = rfd::FileDialog::new().set_file_name("xpacket.xml").save_file() {
                                if let Err(e) = std::fs::write(&path, data) {
                                    self.error_message = Some(format!("Failed to save file: {}",e));
                                }
                            }
                        } else {
                            self.error_message = Some("There is no xpacket loaded!".to_string());
                        }

                    }
                    if ui.button("About").clicked() {
                        //show info about progeram
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("Expand all").clicked() {
                        //do
                    }
                    if ui.button("Collapse all").clicked() {
                        //do
                    }
                });
            });
        });

        if let Some(path) = self.file_to_load.take() {
            self.load_file(&path);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(root) = &self.root {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        show_node(ui, root);
                    });
            } else {
                //no file loaded
                ui.centered_and_justified(|ui| ui.label("No file loaded."));
            }
        });

        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let (Some(offset), Some(size), Some(current_file)) = (self.xpacket_offset, self.xpacket_size, &self.current_file) {
                    ui.label(format!("File: {} | xpacket - Offset: {}, Size: {}", current_file, offset, size));
                } else {
                    ui.label("...");
                }
            });
        });


        if let Some(msg) = &self.error_message {
            let msg = msg.clone();
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new(msg).strong());
                    if ui.button("OK").clicked() {
                        self.error_message = None;
                    }
            });
        }

    }
}

fn show_node(ui: &mut egui::Ui, node: &XmlNode) {
    egui::CollapsingHeader::new(RichText::new(&node.name).strong().italics())
        .id_salt(node as *const _) //prevent collision of elements with same name
        .default_open(false)
        .show(ui, |ui| {
            for (k, v) in &node.attributes {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}:", k)).strong()); // bold
                    ui.label(v);                         // normal
                });
            }

            if let Some(text) = &node.text {
                ui.label(format!("{}", text));
            }

            for child in &node.children {
                show_node(ui, child);
            }
        });
}
// -- end GUI -- 

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = std::env::args().nth(1); //optional: file path from cmd arg

    eframe::run_native(
        "xmpeek",
        eframe::NativeOptions::default(),
        Box::new(move |_| {
            Ok(Box::new(XmpeekApp {
                root: None,
                error_message: None,
                current_file: None,
                file_to_load: file_path,
                xpacket_data: None,
                xpacket_offset: None,
                xpacket_size: None,
            }))
        }),
    )?;

    Ok(())
}