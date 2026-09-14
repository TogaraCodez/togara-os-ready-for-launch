//! Tensor Library for Neural Networks
//! 
//! Priority 6: AI Runtime
//! Classification: RUNTIME
//! 
//! Basic tensor operations for neural network inference.

/// Tensor (multi-dimensional array)
pub struct Tensor {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

impl Tensor {
    /// Create a new tensor
    pub fn new(shape: Vec<usize>, data: Vec<f32>) -> Self {
        Self { shape, data }
    }

    /// Create a zero-filled tensor
    pub fn zeros(shape: Vec<usize>) -> Self {
        let size = shape.iter().product();
        Self {
            shape,
            data: vec![0.0; size],
        }
    }

    /// Create a one-filled tensor
    pub fn ones(shape: Vec<usize>) -> Self {
        let size = shape.iter().product();
        Self {
            shape,
            data: vec![1.0; size],
        }
    }

    /// Get tensor size (total elements)
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get element at index
    pub fn get(&self, index: usize) -> f32 {
        self.data[index]
    }

    /// Set element at index
    pub fn set(&mut self, index: usize, value: f32) {
        self.data[index] = value;
    }

    /// Matrix multiplication: C = A × B
    /// A: [M × K], B: [K × N], C: [M × N]
    pub fn matmul(a: &Tensor, b: &Tensor) -> Tensor {
        assert_eq!(a.shape.len(), 2);
        assert_eq!(b.shape.len(), 2);
        assert_eq!(a.shape[1], b.shape[0]);
        
        let m = a.shape[0];
        let k = a.shape[1];
        let n = b.shape[1];
        
        let mut c = Tensor::zeros(vec![m, n]);
        
        for i in 0..m {
            for j in 0..n {
                let mut sum = 0.0;
                for p in 0..k {
                    sum += a.get(i * k + p) * b.get(p * n + j);
                }
                c.set(i * n + j, sum);
            }
        }
        
        c
    }

    /// Element-wise addition
    pub fn add(&self, other: &Tensor) -> Tensor {
        assert_eq!(self.size(), other.size());
        
        let data = self.data.iter()
            .zip(other.data.iter())
            .map(|(&a, &b)| a + b)
            .collect();
        
        Tensor {
            shape: self.shape.clone(),
            data,
        }
    }

    /// Element-wise multiplication
    pub fn mul(&self, other: &Tensor) -> Tensor {
        assert_eq!(self.size(), other.size());
        
        let data = self.data.iter()
            .zip(other.data.iter())
            .map(|(&a, &b)| a * b)
            .collect();
        
        Tensor {
            shape: self.shape.clone(),
            data,
        }
    }

    /// Apply ReLU activation
    pub fn relu(&self) -> Tensor {
        let data = self.data.iter()
            .map(|&x| x.max(0.0))
            .collect();
        
        Tensor {
            shape: self.shape.clone(),
            data,
        }
    }

    /// Apply Sigmoid activation
    pub fn sigmoid(&self) -> Tensor {
        let data = self.data.iter()
            .map(|&x| 1.0 / (1.0 + (-x).exp()))
            .collect();
        
        Tensor {
            shape: self.shape.clone(),
            data,
        }
    }

    /// Apply Softmax activation (along last dimension)
    pub fn softmax(&self) -> Tensor {
        let n = self.shape[self.shape.len() - 1];
        let mut data = Vec::with_capacity(self.size());
        
        for i in (0..self.size()).step_by(n) {
            // Find max for numerical stability
            let max_val = self.data[i..i+n].iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            
            // Compute exp and sum
            let mut sum = 0.0;
            let mut exps = Vec::with_capacity(n);
            for j in 0..n {
                let exp_val = (self.data[i + j] - max_val).exp();
                exps.push(exp_val);
                sum += exp_val;
            }
            
            // Normalize
            for exp_val in exps {
                data.push(exp_val / sum);
            }
        }
        
        Tensor {
            shape: self.shape.clone(),
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_creation() {
        let t = Tensor::zeros(vec![2, 3]);
        assert_eq!(t.shape, vec![2, 3]);
        assert_eq!(t.size(), 6);
        assert_eq!(t.get(0), 0.0);
    }

    #[test]
    fn test_tensor_matmul() {
        // A: [2×3], B: [3×2]
        let a = Tensor::new(
            vec![2, 3],
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        );
        let b = Tensor::new(
            vec![3, 2],
            vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0],
        );
        
        let c = Tensor::matmul(&a, &b);
        
        // C should be [2×2]
        assert_eq!(c.shape, vec![2, 2]);
        
        // C[0,0] = 1*7 + 2*9 + 3*11 = 58
        assert_eq!(c.get(0), 58.0);
        // C[0,1] = 1*8 + 2*10 + 3*12 = 64
        assert_eq!(c.get(1), 64.0);
    }

    #[test]
    fn test_relu() {
        let t = Tensor::new(vec![4], vec![-2.0, -1.0, 0.0, 1.0]);
        let r = t.relu();
        
        assert_eq!(r.get(0), 0.0);
        assert_eq!(r.get(1), 0.0);
        assert_eq!(r.get(2), 0.0);
        assert_eq!(r.get(3), 1.0);
    }
}
