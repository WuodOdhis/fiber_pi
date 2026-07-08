use crate::{Error, Result};

pub fn parse_amount(value: &str) -> Result<u128> {
    if let Some(hex) = value.strip_prefix("0x") {
        u128::from_str_radix(hex, 16).map_err(|_| Error::InvalidAmount(value.to_string()))
    } else {
        value
            .parse::<u128>()
            .map_err(|_| Error::InvalidAmount(value.to_string()))
    }
}

pub fn to_hex_amount(value: u128) -> String {
    format!("0x{value:x}")
}

pub fn calculate_fee(gross_amount: u128, fee_rate_bps: u128) -> u128 {
    gross_amount.saturating_mul(fee_rate_bps) / 10_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_decimal_and_hex_amounts() {
        assert_eq!(parse_amount("1000").unwrap(), 1000);
        assert_eq!(parse_amount("0x3e8").unwrap(), 1000);
    }

    #[test]
    fn calculates_basis_point_fee() {
        assert_eq!(calculate_fee(1000, 100), 10);
        assert_eq!(calculate_fee(1000, 25), 2);
    }
}
