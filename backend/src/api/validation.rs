use serde_json::json;

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct ValidationErrors {
    errors: Vec<ValidationError>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ValidationError {
    field: &'static str,
    message: String,
}

pub trait ValidateRequest {
    fn validate(&self) -> Result<(), ValidationErrors>;
}

impl ValidationErrors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, field: &'static str, message: impl Into<String>) {
        self.errors.push(ValidationError {
            field,
            message: message.into(),
        });
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn into_result(self) -> Result<(), Self> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(self)
        }
    }

    pub fn into_json(self) -> serde_json::Value {
        json!({ "fields": self.errors })
    }
}

pub fn require_non_empty(errors: &mut ValidationErrors, field: &'static str, value: &str) {
    if value.trim().is_empty() {
        errors.push(field, "must not be empty");
    }
}

pub fn require_max_len(
    errors: &mut ValidationErrors,
    field: &'static str,
    value: &str,
    max_len: usize,
) {
    if value.chars().count() > max_len {
        errors.push(field, format!("must be at most {max_len} characters"));
    }
}

#[cfg(test)]
mod tests {
    use super::{require_max_len, require_non_empty, ValidationErrors};

    #[test]
    fn validation_errors_collect_field_messages() {
        let mut errors = ValidationErrors::new();

        require_non_empty(&mut errors, "name", " ");
        require_max_len(&mut errors, "summary", "abcdef", 5);

        assert!(errors.into_result().is_err());
    }
}
