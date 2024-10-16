use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserValues {
    display_name: Option<Option<String>>,
    email: Option<String>,
    password: Option<String>,
}

impl UpdateUserValues {
    /// Create an empty instance that updates no fields.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the display name of the user. If `None` is passed, the display name will be removed.
    pub fn display_name(mut self, display_name: Option<String>) -> Self {
        self.display_name = Some(display_name);
        self
    }

    /// Update the user's email.
    pub fn email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }

    /// Update the user's password.
    pub fn password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateUserBody<'a> {
    local_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    delete_attribute: Vec<&'static str>,
}

impl<'a> UpdateUserBody<'a> {
    pub(crate) fn from_values(user_id: &'a str, values: UpdateUserValues) -> Self {
        // We need to specify a list of attributes to delete explicitly according to
        // the Firebase Node.js Admin SDK implementation: https://github.com/firebase/firebase-admin-node/blob/f1c55238a885a76b5225fe5bdaa580c7ae1cc8a4/src/auth/auth-api-request.ts#L1418-L1436
        let mut delete_attribute = Vec::new();

        if let Some(None) = values.display_name {
            delete_attribute.push("DISPLAY_NAME");
        }

        Self {
            local_id: user_id,
            display_name: values.display_name.flatten(),
            email: values.email,
            password: values.password,
            delete_attribute,
        }
    }
}
