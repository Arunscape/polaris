use thiserror::Error;

#[derive(Error, Debug)]
pub enum APIError {
	#[error("Incorrect Credentials")]
	IncorrectCredentials,
	#[error("Cannot remove own admin privilege")]
	OwnAdminPrivilegeRemoval,
	#[error("Unspecified")]
	Unspecified,
}

impl From<anyhow::Error> for APIError {
	fn from(_: anyhow::Error) -> Self {
		APIError::Unspecified
	}
}
