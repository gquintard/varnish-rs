varnish::boilerplate!();

#[allow(non_camel_case_types)]
mod vmod;

#[cfg(test)]
mod tests {
    varnish::vtc!(test01);
}
