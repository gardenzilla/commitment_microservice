use chrono::Utc;
use gzlib::proto::commitment::{CommitmentInfo, CommitmentObj, CustomerObj, PurchaseInfo};

use crate::commitment::CommitmentExt;

pub enum ServiceError {
  InternalError(String),
  NotFound(String),
  AlreadyExists(String),
  BadRequest(String),
}

impl ServiceError {
  pub fn internal_error(msg: &str) -> Self {
    ServiceError::InternalError(msg.to_string())
  }
  pub fn not_found(msg: &str) -> Self {
    ServiceError::NotFound(msg.to_string())
  }
  pub fn already_exist(msg: &str) -> Self {
    ServiceError::AlreadyExists(msg.to_string())
  }
  pub fn bad_request(msg: &str) -> Self {
    ServiceError::BadRequest(msg.to_string())
  }
}

impl std::fmt::Display for ServiceError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ServiceError::InternalError(msg) => write!(f, "{}", msg),
      ServiceError::NotFound(msg) => write!(f, "{}", msg),
      ServiceError::AlreadyExists(msg) => write!(f, "{}", msg),
      ServiceError::BadRequest(msg) => write!(f, "{}", msg),
    }
  }
}

impl std::fmt::Debug for ServiceError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("")
      .field(&"ServiceError".to_string())
      .field(self)
      .finish()
  }
}

impl From<ServiceError> for ::tonic::Status {
  fn from(error: ServiceError) -> Self {
    match error {
      ServiceError::InternalError(msg) => ::tonic::Status::internal(msg),
      ServiceError::NotFound(msg) => ::tonic::Status::not_found(msg),
      ServiceError::AlreadyExists(msg) => ::tonic::Status::already_exists(msg),
      ServiceError::BadRequest(msg) => ::tonic::Status::invalid_argument(msg),
    }
  }
}

impl From<::packman::PackError> for ServiceError {
  fn from(error: ::packman::PackError) -> Self {
    match error {
      ::packman::PackError::ObjectNotFound => ServiceError::not_found(&error.to_string()),
      _ => ServiceError::internal_error(&error.to_string()),
    }
  }
}

pub type ServiceResult<T> = Result<T, ServiceError>;

impl From<std::env::VarError> for ServiceError {
  fn from(error: std::env::VarError) -> Self {
    ServiceError::internal_error(&format!("ENV KEY NOT FOUND. {}", error))
  }
}

impl From<crate::commitment::PurchaseInfo> for PurchaseInfo {
  fn from(f: crate::commitment::PurchaseInfo) -> Self {
    Self {
      purchase_id: f.purchase_id.to_string(),
      total_net: f.total_net,
      total_gross: f.total_gross,
      applied_discount: f.applied_discount,
      removed: f.removed,
      created_at: f.crated_at.to_rfc3339(),
    }
  }
}

impl From<crate::commitment::Commitment> for CommitmentObj {
  fn from(f: crate::commitment::Commitment) -> Self {
    Self {
      commitment_id: f.commitment_id.to_string(),
      customer_id: f.customer_id,
      target: f.target,
      discount_percentage: f.discount_percentage,
      valid_till: f.valid_till.to_rfc3339(),
      balance: f.balance,
      purchase_log: f
        .purchase_log
        .iter()
        .map(|pi| pi.clone().into())
        .collect::<Vec<PurchaseInfo>>(),
      is_withdrawn: f.is_withdrawn(),
      is_active: f.is_active(),
      created_at: f.created_at.to_rfc3339(),
      created_by: f.created_by,
    }
  }
}

impl From<crate::commitment::Commitment> for CommitmentInfo {
  fn from(f: crate::commitment::Commitment) -> Self {
    Self {
      commitment_id: f.commitment_id.to_string(),
      customer_id: f.customer_id,
      target: f.target,
      discount_percentage: f.discount_percentage,
      balance: f.balance,
      is_active: f.is_active(),
    }
  }
}

impl From<crate::commitment::Customer> for CustomerObj {
  fn from(f: crate::commitment::Customer) -> Self {
    Self {
      customer_id: f.customer_id,
      commitments: f
        .commitments
        .iter()
        .map(|c| c.clone().into())
        .collect::<Vec<CommitmentObj>>(),
    }
  }
}
