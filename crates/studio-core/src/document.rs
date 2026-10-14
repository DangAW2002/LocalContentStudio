use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ElementType {
    Text,
    Image,
    Shape,
    Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ElementSemanticRole {
    Headline,
    ProductImage,
    Cta,
    BrandLogo,
    Statistic,
    Quote,
    SocialProof,
    Footer,
    Body,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_align: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: String,
    #[serde(rename = "type")]
    pub element_type: ElementType,
    pub semantic_role: ElementSemanticRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_ref: Option<String>,
    pub box_model: BoundingBox,
    pub style: ElementStyle,
    pub locked: bool,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtboardBackground {
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artboard {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub background: ArtboardBackground,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrandRef {
    pub id: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateBinding {
    pub template_id: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreativeDocument {
    pub schema_version: String,
    pub id: String,
    pub brand_ref: Option<BrandRef>,
    pub artboards: Vec<Artboard>,
    pub template_binding: Option<TemplateBinding>,
}

impl CreativeDocument {
    pub const SCHEMA_V1: &'static str = "creative-document/v1";

    pub fn new(id: impl Into<String>) -> Self {
        Self {
            schema_version: Self::SCHEMA_V1.to_string(),
            id: id.into(),
            brand_ref: None,
            artboards: Vec::new(),
            template_binding: None,
        }
    }

    pub fn compute_hash(&self) -> String {
        let json_bytes = serde_json::to_vec(self).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(&json_bytes);
        hex::encode(hasher.finalize())
    }
}
