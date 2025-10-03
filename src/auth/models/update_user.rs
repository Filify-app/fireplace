use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserValues {
    display_name: Option<Option<String>>,
    email: Option<String>,
    password: Option<String>,
    disabled: Option<bool>,
}

impl UpdateUserValues {
    /// Create an empty instance that updates no fields.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the display name of the user. If `None` is passed, the display name will be removed.
    pub fn display_name(mut self, display_name: Option<impl Into<String>>) -> Self {
        self.display_name = Some(display_name.map(Into::into));
        self
    }

    /// Update the user's email.
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Update the user's password.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Enable or disable the user.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
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
    #[serde(skip_serializing_if = "Option::is_none")]
    disable_user: Option<bool>,
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
            // The `disabled` field is internally renamed to `disableUser`. See the Firebase Node
            // Admin SDK implementation for reference:
            //   - https://github.com/firebase/firebase-admin-node/blob/137a0d9312b0b45b69f6a5111081420729d8eaeb/src/auth/auth-api-request.ts#L480-L486
            //   - https://github.com/firebase/firebase-admin-node/blob/137a0d9312b0b45b69f6a5111081420729d8eaeb/src/auth/auth-api-request.ts#L1468-L1472
            disable_user: values.disabled,
        }
    }
}
