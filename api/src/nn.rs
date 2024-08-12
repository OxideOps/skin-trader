use burn::module::Module;
use burn::nn::{Linear, LinearConfig, Relu};
use burn::tensor::{backend::Backend, Distribution, Tensor};

#[derive(Module, Debug)]
struct Nn<B: Backend> {
    layer1: Linear<B>,
    activation: Relu,
    layer2: Linear<B>,
}

impl<B: Backend> Nn<B> {
    pub fn new(
        input_size: usize,
        hidden_size: usize,
        output_size: usize,
        device: &B::Device,
    ) -> Self {
        Self {
            layer1: LinearConfig::new(input_size, hidden_size).init(device),
            activation: Relu::new(),
            layer2: LinearConfig::new(hidden_size, output_size).init(device),
        }
    }

    pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = self.layer1.forward(input);
        let x = self.activation.forward(x);
        self.layer2.forward(x)
    }
}

fn computation<B: Backend>() {
    // Create the device where to do the computation
    let device = B::Device::default();

    // Initialize the neural network
    let net = Nn::<B>::new(10, 5, 2, &device);

    // Create a random input tensor
    let input: Tensor<B, 2> = Tensor::random([3, 10], Distribution::Uniform(-1.0, 1.0), &device);

    // Perform a forward pass
    let output = net.forward(input);

    // Print the output
    println!("Network output:\n{:}", output);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_computation() {
        computation::<burn::backend::NdArray>();
    }
}
