//for no cmd on windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, TopBottomPanel, MenuBar, RichText};
use rfd::{FileDialog, MessageDialog, MessageLevel};

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
    current_file: Option<String>,
    file_to_load: Option<String>,
    xpacket_info: Option<XpacketInfo>,
}

impl XmpeekApp {
    fn load_file(&mut self, path: &str) -> Result<(), String> {
        let data = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
        let info = extract_xpacket(&data).map_err(|e| format!("Failed to extract xpacket: {}", e))?;
        
        let xml = String::from_utf8_lossy(&info.data);
        let doc = roxmltree::Document::parse(&xml).map_err(|e| format!("Failed to parse XML: {}", e))?;
        self.root = Some(build_tree(doc.root_element()).unwrap());

        self.current_file = Some(path.to_string());
        self.xpacket_info = Some(info);
        
        Ok(())
    }
}

impl eframe::App for XmpeekApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        //open a file
                        if let Some(path) = FileDialog::new().pick_file() {
                            if let Err(err) = self.load_file(path.to_str().unwrap()) {
                                MessageDialog::new().set_level(MessageLevel::Error).set_title("Error").set_description(err).show();
                            }
                        }
                    }
                    if ui.button("Save xpacket").clicked() {
                        //export the xpacket as a file
                        if let Some(info) = &self.xpacket_info {
                            if let Some(path) = FileDialog::new().set_file_name("xpacket.xml").save_file() {
                                if let Err(e) = std::fs::write(&path, &info.data) {
                                    MessageDialog::new().set_level(MessageLevel::Error).set_title("Error").set_description(format!("Failed to save file: {}", e)).show();
                                }
                            }
                        } else {
                            MessageDialog::new().set_level(MessageLevel::Error).set_title("Error").set_description("There is no xpacket loaded!").show();
                        }

                    }
                    if ui.button("About").clicked() {
                        //show info about progeram
                        MessageDialog::new().set_level(MessageLevel::Info).set_title("About").set_description(format!("xmpeek")).show();
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
            if let Err(err) = self.load_file(&path) {
                MessageDialog::new().set_level(MessageLevel::Error).set_title("Error").set_description(err).show();
                //if we launched file from cmdline and it failed, do NOT display the app gui. at least this is the behaviour i want - so exit the app
                std::process::exit(0);
            };
        }

        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let (Some(info), Some(current_file)) = (&self.xpacket_info, &self.current_file) {
                    ui.label(format!("File: {} | xpacket - Offset: {}, Size: {}", current_file, info.offset, info.size));
                } else {
                    ui.label("...");
                }
            });
        });

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
                current_file: None,
                file_to_load: file_path,
                xpacket_info: None,
            }))
        }),
    )?;

    Ok(())
}