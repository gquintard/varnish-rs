use varnish_sys::ffi::vsc_seg;

pub struct VscMetricDef<'a> {
    pub name: &'a str,
    pub counter_type: &'a str, // "counter", "gauge", etc
    pub ctype: &'a str,        // "uint64_t", etc
    pub level: &'a str,        // "info", "debug", etc
    pub oneliner: &'a str,     // "A counter", "A gauge", etc
    pub format: &'a str,       // "integer", "bytes", "duration", "bitmap",etc
    pub docs: &'a str,
}

pub trait VscCounterStruct: Sized {
    fn get_struct_metrics() -> Vec<VscMetricDef<'static>>;
    fn new(module_name: &str, module_prefix: &str) -> Self;
    fn set_vsc_seg(&mut self, seg: *mut vsc_seg);
    fn drop(&mut self);

    fn build_json(module_name: &str) -> String {
        let metrics = Self::get_struct_metrics();
        let mut elem_json = String::new();

        for (i, metric) in metrics.iter().enumerate() {
            if i > 0 {
                elem_json.push_str(",\n");
            }
            elem_json.push_str(&format!(
                r#"    "{}": {{
      "type": "{}",
      "ctype": "{}",
      "level": "{}",
      "oneliner": "{}",
      "format": "{}",
      "index": {},
      "name": "{}",
      "docs": "{}"
    }}"#,
                metric.name,
                metric.counter_type,
                metric.ctype,
                metric.level,
                metric.oneliner,
                metric.format,
                i * 8,
                metric.name,
                metric.docs
            ));
        }

        format!(
            r#"{{
  "version": "1",
  "name": "{}",
  "oneliner": "{} statistics",
  "order": 100,
  "docs": "",
  "elements": {},
  "elem": {{
{}
  }}
}}"#,
            module_name,
            module_name,
            metrics.len(),
            elem_json
        )
    }
}
