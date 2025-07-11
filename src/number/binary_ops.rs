use std::iter::from_fn;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

use crate::number::Number;
use crate::sum_result::SumResult;
use crate::trit::Trit;

impl <const N: usize> Add for Number<N> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        // Zip trits from both operands, going in reverse order so carries can propagate upwards
        let reverse_result_trits = self.0.iter().rev()
            .zip(rhs.0.iter().rev())
            // "Scan" as we need an output at each index, with accumulator propagating the carry trit
            .scan(Trit::ZERO, |carry, (lhs, rhs)| {
                let SumResult{result, carry: new_carry} = lhs.add_with_carry(rhs, carry);
                *carry = new_carry;
                Some(result)
            });
        
        Number::<N>::from_rev_iter(reverse_result_trits)
    }
}

impl <const N: usize> AddAssign for Number<N> {
    fn add_assign(&mut self, rhs: Self) {
        // Much the same as the Add trait, but mutating self data in-place. As such we replace
        // "scan" with "for_each" to remove need for output. However we then lose the accumulator
        // So we need to declare an external `carry` variable.
        let mut carry = Trit::ZERO;
        self.0.iter_mut().rev()
            .zip(rhs.0.iter().rev())
            .for_each(|(lhs, rhs)| {
                let SumResult { result, carry: new_carry} = lhs.add_with_carry(rhs, &carry);
                carry = new_carry;
                *lhs = result;
            });
    }
}

impl <const N: usize> AddAssign<Trit> for Number<N> {
    fn add_assign(&mut self, rhs: Trit) {
        // Add the rhs to the least significant trit. Keep performing additions
        // and propagating carries through the indices until we don't need to
        // carry anymore or we run out of trit indices.
        let mut carry = rhs;
        for trit in self.0.iter_mut().rev() {
            if carry == Trit::ZERO {break;}

            let SumResult{result, carry: new_carry} = trit.add(&carry);
            carry = new_carry;
            *trit = result;
        }
    }
}

impl <const N: usize> Sub for Number<N> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + -rhs
    }
}

impl <const N: usize> SubAssign for Number<N> {
    fn sub_assign(&mut self, rhs: Self) {
        *self += -rhs;
    }
}


impl <const N: usize> Mul for Number<N> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        // Generator that will provide continually left-shifted copies
        // of the rhs operand. This will support the shift-and-add
        // approach of the multiplication.
        let mut rhs_shifted = rhs;
        let rhs_shifter = move || {
            let out = rhs_shifted;
            rhs_shifted <<= 1;
            Some(out)
        };

        self.0.iter().rev()
            .zip(from_fn(rhs_shifter))
            .filter_map(|(current_trit, rhs_shifted)| 
                match current_trit {
                    Trit::NEG => Some(-rhs_shifted),
                    Trit::ZERO => None,
                    Trit::POS => Some(rhs_shifted)
                }
            )
            .sum()
    }
}

impl <const N: usize> MulAssign for Number<N> {
    fn mul_assign(&mut self, rhs: Self) {
        // Based on the shift-and-add approach to multiplication I don't see an
        // obvious way to do a more efficient in-place multiplication operator.
        // We need a third spare variable to build the complete result and then
        // copy it into our own array, or otherwise we make a copy of our own 
        // array before zeroing it out and perform the shift-and-add in-place
        // on that array.

        *self = *self * rhs;
    }
}

impl <const N: usize> Div for Number<N> {
    type Output = Self;

    fn div(self, divisor: Self) -> Self::Output {
        if divisor == Number::<N>::ZERO {
            panic!("Attempt to divide by zero")
        }

        // Integer division implemented with a repeated subtraction approach. We
        // convert numerator and divisor to positive to perform the division, and
        // then decide whether to flip the result based on if they originally had
        // different signs.

        let numerator_is_negative = self < Number::<N>::ZERO;
        let mut abs_remainder = if numerator_is_negative {-self} else {self};

        let divisor_is_negative = divisor < Number::<N>::ZERO;
        let abs_divisor = if divisor_is_negative {-divisor} else {divisor};

        let mut quotient = Number::<N>::ZERO;
        while abs_remainder >= abs_divisor {
            abs_remainder -= abs_divisor;
            quotient.inc();
        }

        if numerator_is_negative ^ divisor_is_negative {
            -quotient
        } else {
            quotient
        }
    }
}

impl <const N: usize> DivAssign for Number<N> {
    fn div_assign(&mut self, divisor: Self) {
        *self = *self / divisor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_operations() {
        let num_23 = Number::<8>::from("+0--");
        let num_33 = Number::<8>::from("++-0");

        assert_eq!(num_23 + num_33, Number::<8>::from("+-0+-")); // Sum to 56
        assert_eq!(num_23 - num_33, Number::<8>::from("-0-")); // Difference is -10
        assert_eq!(num_33 - num_23, Number::<8>::from("+0+")); // Difference is 10
        assert_eq!(num_23 * num_33, Number::<8>::from("+00+0+0")); // Product is 759
    }

    #[test]
    fn in_place_binary_operations() {
        let num_23 = Number::<8>::from("+0--");
        let num_33 = Number::<8>::from("++-0");

        let mut temp = num_23;
        temp += num_33;
        assert_eq!(temp, Number::<8>::from("+-0+-")); // Sum to 56

        temp = num_23;
        temp -= num_33;
        assert_eq!(temp, Number::<8>::from("-0-")); // Difference is -10
        
        temp = num_33;
        temp -= num_23;
        assert_eq!(temp, Number::<8>::from("+0+")); // Difference is 10
        
        temp = num_23;
        temp *= num_33;
        assert_eq!(temp, Number::<8>::from("+00+0+0")); // Product is 759
}

    #[test]
    fn integer_division() {
        let num_59 = Number::<8>::from("+-+--");
        let num_60 = Number::<8>::from("+-+-0");
        let num_61 = Number::<8>::from("+-+-+");
        let num_12 = Number::<8>::from("++0");

        // Integral division with remainders discarded
        assert_eq!(num_59 / num_12, Number::<8>::from("0++")); // 59 / 12 = 4
        assert_eq!(num_60 / num_12, Number::<8>::from("+--")); // 60 / 12 = 5
        assert_eq!(num_61 / num_12, Number::<8>::from("+--")); // 61 / 12 = 5

        // Negatively signed numerators and divisors, results rounded towards zero
        assert_eq!(-num_59 /  num_12, Number::<8>::from("0--")); // -59 /  12 = -4
        assert_eq!( num_59 / -num_12, Number::<8>::from("0--")); //  59 / -12 = -4
        assert_eq!(-num_59 / -num_12, Number::<8>::from("0++")); // -59 / -12 =  4

        // Dividing zero by any number results in zero
        let num_0: Number<8> = Number::<8>::ZERO;

        assert_eq!(num_0 / num_60, num_0);    // 0 /  60 = 0
        assert_eq!(num_0 / (-num_60), num_0); // 0 / -60 = 0
    }

    #[test]
    #[should_panic(expected = "Attempt to divide by zero")]
    fn pos_divide_by_zero() {
        let num_61 = Number::<8>::from("+-+-+");
        let num_0: Number<8> = Number::<8>::ZERO;

        let _ = num_61 / num_0;
    }

    #[test]
    #[should_panic(expected = "Attempt to divide by zero")]
    fn neg_divide_by_zero() {
        let num_neg_61 = Number::<8>::from("-+-+-");
        let num_0: Number<8> = Number::<8>::ZERO;

        let _ = num_neg_61 / num_0;
    }  
}