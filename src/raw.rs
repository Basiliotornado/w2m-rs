pub struct Raw {
    pub data: Vec<f64>,
}

impl Raw {
    pub fn new_i16(input: Vec<u8>) -> Self {
        type T = i16;
        const SIZE: usize = size_of::<T>();

        if !input.len().is_multiple_of(SIZE) {
            panic!("Raw array is wrong size!");
        }

        let samples = input.len() / SIZE;
        let mut data = vec![0f64; samples];
        // let reader = Reader::new(std::io::stdin());

        for i in 0..samples {
            data[i] = i16::from_le_bytes(input[i * SIZE..(i + 1) * SIZE].try_into().unwrap())
                as f64
                / T::MAX as f64;
        }

        Self { data }
    }
}