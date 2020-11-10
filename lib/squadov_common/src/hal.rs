use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct HalLink {
    href: String
}

#[derive(Serialize)]
pub struct HalResponse<T> {
    #[serde(rename = "_links")]
    links: HashMap<String, HalLink>,
    data: T
}

impl<T> HalResponse<T> {
    pub fn new(data: T) -> HalResponse<T> {
        HalResponse{
            links: HashMap::new(),
            data: data,
        }
    }

    pub fn add_link(&mut self, title: &str, href: &str) -> &mut HalResponse<T> {
        self.links.insert(String::from(title), HalLink{
            href: String::from(href),
        });
        
        self
    }
}