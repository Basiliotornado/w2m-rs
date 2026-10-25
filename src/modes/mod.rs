pub mod fft;
pub mod zero_cross;

#[derive(Debug)]
pub enum Modes {
    Multispectra = 0,
    ZeroCross = 1,
}