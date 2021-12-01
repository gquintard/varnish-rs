varnish::boilerplate!();

mod vmod;

#[cfg(test)]
mod tests {
    varnish::vtc!(test01);
}
