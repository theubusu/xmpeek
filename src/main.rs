use std::env;
use std::fs;
//use std::io::{Write};
use eframe::egui::{self, TopBottomPanel, MenuBar, RichText};

const XPACKET_BEGIN: &[u8] = b"<?xpacket begin=";
const XPACKET_END_START: &[u8] = b"<?xpacket end=";
const XPACKET_END: &[u8] = b"?>";

fn find_bytes(data: &[u8], pattern: &[u8]) -> Option<usize> {
    data.windows(pattern.len()).position(|window| window == pattern)
}

//xml
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
//

//GUI
struct XmpeekApp {
    root: XmlNode,
}

impl eframe::App for XmpeekApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        //do
                    }
                    if ui.button("Save xpacket").clicked() {
                        //do
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui.button("1").clicked() {
                        //do
                    }
                    if ui.button("2").clicked() {
                        //do
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    show_node(ui, &self.root);
                });
        });
    }
}

fn show_node(ui: &mut egui::Ui, node: &XmlNode) {
    egui::CollapsingHeader::new(&node.name)
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
                ui.label(format!("text: {}", text));
            }

            for child in &node.children {
                show_node(ui, child);
            }
        });
}
//

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).ok_or("Ussage: xmpeek <file>")?;
    let data = fs::read(&path)?;

    //locate xpacket
    let begin = find_bytes(&data, XPACKET_BEGIN).ok_or("No xpacket found!")?;
    println!("xpacket begin found at {}", begin);
    let end_start = find_bytes(&data[begin..], XPACKET_END_START).ok_or("xpacket end start not found!")? + begin;
    let end = find_bytes(&data[end_start..], XPACKET_END).ok_or("xpacket end not found!")? + end_start + XPACKET_END.len();

    let xpacket = &data[begin..end].to_vec();
    println!("xpacket OK - Start: {}, End: {}, Lenght: {}", begin, end, xpacket.len() );
    drop(data); //drop the file data to save memory

    //let mut out = fs::File::create("out.xml")?;
    //out.write_all(&xpacket)?;
    //println!("- Saved file!");

    let xml = String::from_utf8_lossy(&xpacket);
    let doc = roxmltree::Document::parse(&xml)?;
    let root = build_tree(doc.root_element()).unwrap();

    eframe::run_native(
        "xmpeek",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(XmpeekApp { root }))),
    )?;

    Ok(())
}
