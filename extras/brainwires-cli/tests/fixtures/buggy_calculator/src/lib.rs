/// A simple calculator library with basic arithmetic operations
pub struct Calculator;

impl Calculator {
    /// Create a new calculator instance
    pub fn new() -> Self {
        Self
    }

    /// Add two numbers
    pub fn add(&self, a: f64, b: f64) -> f64 {
        a + b
    }

    /// Subtract b from a
    pub fn subtract(&self, a: f64, b: f64) -> f64 {
        a - b
    }

    /// Multiply two numbers
    pub fn multiply(&self, a: f64, b: f64) -> f64 {
        a * b
    }

    /// Divide a by b
    /// BUG: This function has the wrong operator!
    pub fn divide(&self, a: f64, b: f64) -> Result<f64, String> {
        if b == 0.0 {
            Err("Cannot divide by zero".to_string())
        } else {
            // BUG: Should be a / b, not a * b
            Ok(a * b)
        }
    }

    /// Calculate the average of a list of numbers
    pub fn average(&self, numbers: &[f64]) -> Result<f64, String> {
        if numbers.is_empty() {
            return Err("Cannot calculate average of empty list".to_string());
        }

        let sum: f64 = numbers.iter().sum();
        // BUG: This uses the buggy divide function
        self.divide(sum, numbers.len() as f64)
    }
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let calc = Calculator::new();
        assert_eq!(calc.add(2.0, 3.0), 5.0);
        assert_eq!(calc.add(-1.0, 1.0), 0.0);
    }

    #[test]
    fn test_subtract() {
        let calc = Calculator::new();
        assert_eq!(calc.subtract(5.0, 3.0), 2.0);
        assert_eq!(calc.subtract(3.0, 5.0), -2.0);
    }

    #[test]
    fn test_multiply() {
        let calc = Calculator::new();
        assert_eq!(calc.multiply(2.0, 3.0), 6.0);
        assert_eq!(calc.multiply(-2.0, 3.0), -6.0);
    }

    #[test]
    fn test_divide() {
        let calc = Calculator::new();
        // This test will FAIL due to the bug
        assert_eq!(calc.divide(6.0, 2.0).unwrap(), 3.0);
        assert_eq!(calc.divide(10.0, 5.0).unwrap(), 2.0);
    }

    #[test]
    fn test_divide_by_zero() {
        let calc = Calculator::new();
        assert!(calc.divide(5.0, 0.0).is_err());
    }

    #[test]
    fn test_average() {
        let calc = Calculator::new();
        // This test will also FAIL due to the division bug
        assert_eq!(calc.average(&[2.0, 4.0, 6.0]).unwrap(), 4.0);
        assert_eq!(calc.average(&[10.0, 20.0, 30.0]).unwrap(), 20.0);
    }

    #[test]
    fn test_average_empty() {
        let calc = Calculator::new();
        assert!(calc.average(&[]).is_err());
    }
}
