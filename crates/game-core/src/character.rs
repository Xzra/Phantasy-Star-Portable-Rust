//! Cosmetic character identity. Appearance never changes collision or combat stats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    pub name: String,
    pub armor: u8,
    pub accent: u8,
    pub skin: u8,
    pub hair: u8,
    /// 0: helmet, 1: cropped hair, 2: spiky hair.
    pub head: u8,
    /// 0: slim, 1: balanced, 2: broad.
    pub build: u8,
}
impl Default for Character {
    fn default() -> Self {
        Self {
            name: "Hunter".into(),
            armor: 0,
            accent: 0,
            skin: 1,
            hair: 0,
            head: 0,
            build: 1,
        }
    }
}
impl Character {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.name.is_empty()
            || self.name.len() > 20
            || self.name.trim() != self.name
            || !self
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || " -'".contains(c))
            || !self.name.chars().any(|c| c.is_ascii_alphanumeric())
        {
            return Err(
                "Name needs 1-20 letters or numbers; spaces, apostrophes and hyphens are allowed.",
            );
        }
        if self.armor >= 6
            || self.accent >= 6
            || self.skin >= 5
            || self.hair >= 6
            || self.head >= 3
            || self.build >= 3
        {
            return Err("Unknown character appearance choice.");
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_names_and_out_of_range_cosmetics() {
        for name in [
            "",
            "   ",
            " Hunter",
            "Hunter\n",
            "<script>",
            "abcdefghijklmnopqrstu",
        ] {
            assert!(Character {
                name: name.into(),
                ..Character::default()
            }
            .validate()
            .is_err());
        }
        assert!(Character {
            name: "Rin-02 O'Neil".into(),
            ..Character::default()
        }
        .validate()
        .is_ok());
        for invalid in [
            Character {
                head: 3,
                ..Character::default()
            },
            Character {
                skin: 5,
                ..Character::default()
            },
            Character {
                build: 3,
                ..Character::default()
            },
            Character {
                armor: 6,
                ..Character::default()
            },
        ] {
            assert!(invalid.validate().is_err());
        }
    }
}
