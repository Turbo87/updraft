/// Implements the operations shared by all quantity newtypes: the `ZERO`
/// constant, addition and subtraction of same-typed quantities, negation,
/// scaling by `f64`, the dimensionless ratio of two quantities, and `abs`.
#[expect(
    unused_macros,
    reason = "first used by the quantity types added in the following commits"
)]
macro_rules! impl_quantity_ops {
    ($quantity:ty) => {
        impl std::ops::Add for $quantity {
            type Output = Self;

            fn add(self, rhs: Self) -> Self {
                Self(self.0 + rhs.0)
            }
        }

        impl std::ops::AddAssign for $quantity {
            fn add_assign(&mut self, rhs: Self) {
                self.0 += rhs.0;
            }
        }

        impl std::ops::Sub for $quantity {
            type Output = Self;

            fn sub(self, rhs: Self) -> Self {
                Self(self.0 - rhs.0)
            }
        }

        impl std::ops::SubAssign for $quantity {
            fn sub_assign(&mut self, rhs: Self) {
                self.0 -= rhs.0;
            }
        }

        impl std::ops::Neg for $quantity {
            type Output = Self;

            fn neg(self) -> Self {
                Self(-self.0)
            }
        }

        impl std::ops::Mul<f64> for $quantity {
            type Output = Self;

            fn mul(self, rhs: f64) -> Self {
                Self(self.0 * rhs)
            }
        }

        impl std::ops::Mul<$quantity> for f64 {
            type Output = $quantity;

            fn mul(self, rhs: $quantity) -> $quantity {
                rhs * self
            }
        }

        impl std::ops::Div<f64> for $quantity {
            type Output = Self;

            fn div(self, rhs: f64) -> Self {
                Self(self.0 / rhs)
            }
        }

        /// The dimensionless ratio of two quantities.
        impl std::ops::Div for $quantity {
            type Output = f64;

            fn div(self, rhs: Self) -> f64 {
                self.0 / rhs.0
            }
        }

        impl $quantity {
            /// The additive identity.
            pub const ZERO: Self = Self(0.);

            /// The magnitude of the quantity.
            pub const fn abs(self) -> Self {
                Self(self.0.abs())
            }
        }
    };
}

#[expect(
    unused_imports,
    reason = "first used by the quantity types added in the following commits"
)]
pub(crate) use impl_quantity_ops;

/// Implements `Debug` for a quantity as its inner SI value followed by a
/// fixed unit suffix (e.g. `1234.5 m`). The formatter is forwarded, so a
/// precision such as `{:.1?}` is honored.
#[expect(
    unused_macros,
    reason = "first used by the quantity types added in the following commits"
)]
macro_rules! impl_debug_with_unit {
    ($quantity:ty, $unit:literal) => {
        impl std::fmt::Debug for $quantity {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)?;
                f.write_str($unit)
            }
        }
    };
}

#[expect(
    unused_imports,
    reason = "first used by the quantity types added in the following commits"
)]
pub(crate) use impl_debug_with_unit;
