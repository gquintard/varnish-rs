use crate::parser_utils::parse_doc_str;
use serde::Serialize;
use syn::{punctuated::Punctuated, token::Comma, Data, Field, Fields, Type};

type FieldList = Punctuated<Field, Comma>;

#[derive(Serialize, Clone)]
enum CType {
    #[serde(rename = "uint64_t")]
    Uint64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "lowercase")]
enum MetricType {
    Counter,
    Gauge,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "lowercase")]
enum Level {
    Info,
    Diag,
    Debug,
}

impl Default for Level {
    fn default() -> Self {
        Self::Info
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "lowercase")]
enum Format {
    Integer,
    Bitmap,
    Duration,
    Bytes,
}

impl Default for Format {
    fn default() -> Self {
        Self::Integer
    }
}

#[derive(Serialize, Clone)]
struct VscMetricDef {
    pub name: String,
    #[serde(rename = "type")]
    pub metric_type: MetricType,
    pub ctype: CType,
    pub level: Level,
    pub oneliner: String, // "Counts the number of X", etc
    pub format: Format,
    pub docs: String,
    pub index: Option<usize>,
}

#[derive(Serialize)]
struct VscMetadata {
    version: String,
    name: String,
    oneliner: String,
    order: u32,
    docs: String,
    elements: usize,
    elem: std::collections::HashMap<String, VscMetricDef>,
}

pub fn get_struct_fields(data: &Data) -> &FieldList {
    match data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    }
}

pub fn validate_fields(fields: &FieldList) {
    for field in fields {
        match &field.ty {
            Type::Path(path) => {
                let is_atomic_u64 = path
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == "AtomicU64");

                if !is_atomic_u64 {
                    let field_name = field.ident.as_ref().unwrap();
                    panic!("Field {field_name} must be of type AtomicU64");
                }
            }
            _ => panic!("Field types must be AtomicU64"),
        }
    }
}

fn generate_metrics(fields: &FieldList) -> Vec<VscMetricDef> {
    fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let name = field.ident.as_ref().unwrap().to_string();

            let metric_type = if field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("counter"))
            {
                MetricType::Counter
            } else if field.attrs.iter().any(|attr| attr.path().is_ident("gauge")) {
                MetricType::Gauge
            } else {
                panic!("Field {name} must have either #[counter] or #[gauge] attribute")
            };

            let doc_str = parse_doc_str(&field.attrs);
            let mut doc_lines = doc_str.split('\n').filter(|s| !s.is_empty());
            let oneliner = doc_lines.next().unwrap_or_default().to_string();
            let docs = doc_lines.next().unwrap_or_default().to_string();

            let (level, format) = parse_metric_attributes(
                field,
                match metric_type {
                    MetricType::Counter => "counter",
                    MetricType::Gauge => "gauge",
                },
            );

            VscMetricDef {
                name,
                metric_type,
                ctype: CType::Uint64,
                level,
                oneliner,
                format,
                docs,
                index: Some(i * 8),
            }
        })
        .collect()
}

pub fn generate_metadata_json(name: &str, fields: &FieldList) -> String {
    let metrics = generate_metrics(fields);

    let metadata = VscMetadata {
        version: "1".to_string(),
        name: name.to_string(),
        oneliner: format!("{name} statistics"),
        order: 100,
        docs: String::new(),
        elements: metrics.len(),
        elem: metrics
            .iter()
            .map(|m| (m.name.clone(), m.clone()))
            .collect(),
    };

    serde_json::to_string(&metadata).unwrap()
}

fn parse_metric_attributes(field: &Field, metric_type: &str) -> (Level, Format) {
    let mut level = Level::default();
    let mut format = Format::default();

    if let Some(attrs) = field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident(metric_type))
    {
        let _ = attrs.parse_nested_meta(|meta| {
            match meta.path.get_ident().map(ToString::to_string).as_deref() {
                Some("level") => {
                    let level_str = meta.value()?.parse::<syn::LitStr>()?.value();
                    level = match level_str.as_str() {
                        "info" => Level::Info,
                        "diag" => Level::Diag,
                        "debug" => Level::Debug,
                        _ => panic!("Invalid level value for field {}. Must be one of: info, diag, debug", 
                            field.ident.as_ref().unwrap()),
                    };
                }
                Some("format") => {
                    let format_str = meta.value()?.parse::<syn::LitStr>()?.value();
                    format = match format_str.as_str() {
                        "integer" => Format::Integer,
                        "bitmap" => Format::Bitmap,
                        "duration" => Format::Duration,
                        "bytes" => Format::Bytes,
                        _ => panic!("Invalid format value for field {}. Must be one of: integer, bitmap, duration, bytes",
                            field.ident.as_ref().unwrap()),
                    };
                }
                _ => {}
            }
            Ok(())
        });
    }
    (level, format)
}

pub fn has_repr_c(input: &syn::DeriveInput) -> bool {
    input.attrs.iter().any(|attr| {
        if !attr.path().is_ident("repr") {
            return false;
        }

        let Ok(meta) = attr.parse_args::<syn::Meta>() else {
            return false;
        };

        matches!(meta, syn::Meta::Path(path) if path.is_ident("C"))
    })
}
