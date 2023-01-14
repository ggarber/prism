pub trait Transport {
    fn close(&self);
}

pub struct WebTransport {

}

impl Transport for WebTransport {
    fn close(&self) {
        unimplemented!()
    }
}
