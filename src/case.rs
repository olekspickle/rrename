use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// Unfortunately this duplication is needed,
/// because strum affects ToString conversions and serde affects only serialization:
/// Strum serialize_all will lead for this to be lowecase whereas serde rename_all will not.
/// With serialization it's right the opposite.
/// [github issue](<https://github.com/Peternator7/strum/issues/278>)
#[derive(Debug, Clone, Default, Display, PartialEq, Eq, EnumString, Serialize, Deserialize)]
pub enum Case {
    #[default]
    #[serde(alias = "kebab-case")]
    #[strum(to_string = "kebab-case")]
    Kebab,
    #[serde(alias = "PascalCase")]
    #[strum(to_string = "PascalCase")]
    Pascal,
    #[serde(alias = "snake_case")]
    #[strum(to_string = "snake_case")]
    Snake,
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    const KEBAB: &str = r#"{"somecase":"kebab-case"}"#;
    const PASCAL: &str = r#"{"somecase":"PascalCase"}"#;
    const SNAKE: &str = r#"{"somecase":"snake_case"}"#;

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
    struct Test {
        pub somecase: Case,
    }

    #[test]
    fn simple_case() {
        let kebab = Case::Kebab;
        expect!["kebab-case"].assert_eq(&kebab.to_string());

        let kebab: Case = "kebab-case".try_into().unwrap();
        expect!["kebab-case"].assert_eq(&kebab.to_string());

        let pascal: Case = "PascalCase".try_into().unwrap();
        expect!["PascalCase"].assert_eq(&pascal.to_string());
    }

    #[test]
    fn serde_from_and_to_json() {
        let test: Test = serde_json::from_str(KEBAB).unwrap();
        expect![[r#"
            Test {
                somecase: Kebab,
            }
        "#]]
        .assert_debug_eq(&test);
        expect![[r#"{"somecase":"Kebab"}"#]].assert_eq(&serde_json::to_string(&test).unwrap());

        let test: Test = serde_json::from_str(PASCAL).unwrap();
        expect![[r#"
            Test {
                somecase: Pascal,
            }
        "#]]
        .assert_debug_eq(&test);
        expect![[r#"{"somecase":"Pascal"}"#]].assert_eq(&serde_json::to_string(&test).unwrap());

        let test: Test = serde_json::from_str(SNAKE).unwrap();
        expect![[r#"
            Test {
                somecase: Snake,
            }
        "#]]
        .assert_debug_eq(&test);
        expect![[r#"{"somecase":"Snake"}"#]].assert_eq(&serde_json::to_string(&test).unwrap());
    }
}
