use commitment::{CommitmentExt, CustomerExt};
use gzlib::proto::{
  self,
  commitment::{
    commitment_server::{Commitment, CommitmentServer},
    AddCommitmentRequest, AddPurchaseRequest, CommitmentInfo, CustomerBulkRequest, CustomerRequest,
    RemovePurchaseRequest,
  },
};
use packman::VecPack;
use packman::*;
use prelude::*;
use proto::commitment::{CommitmentInfoResponse, CustomerIds, CustomerObj};
use std::error::Error;
use std::path::PathBuf;
use std::{env, str::FromStr};
use tokio::sync::{oneshot, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

mod commitment;
mod prelude;

struct CommitmentService {
  commitments: Mutex<VecPack<commitment::Customer>>,
}

impl CommitmentService {
  fn init(commitments: VecPack<commitment::Customer>) -> Self {
    Self {
      commitments: Mutex::new(commitments),
    }
  }

  /// Get all customer IDs
  async fn get_customer_ids(&self) -> ServiceResult<Vec<u32>> {
    let res = self
      .commitments
      .lock()
      .await
      .iter()
      .map(|c| c.unpack().customer_id)
      .collect::<Vec<u32>>();
    Ok(res)
  }

  /// Add commitment
  async fn add_commitment(&self, r: AddCommitmentRequest) -> ServiceResult<CustomerObj> {
    // If we have a related customer object
    if let Ok(customer) = self.commitments.lock().await.find_id_mut(&r.customer_id) {
      let res = customer
        .as_mut()
        .unpack()
        .add_commitment(r.target, r.discount_percentage, r.created_by)
        .map_err(|e| ServiceError::bad_request(&e))?
        .clone();
      return Ok(res.into());
    }

    // Otherwise create a new customer
    let new_customer =
      commitment::Customer::new(r.customer_id, r.target, r.discount_percentage, r.created_by)
        .map_err(|e| ServiceError::bad_request(&e))?;

    // Insert to customer commitments DB
    self.commitments.lock().await.insert(new_customer)?;

    // Re-query it and return
    let res = self
      .commitments
      .lock()
      .await
      .find_id(&r.customer_id)?
      .unpack()
      .clone();

    // Return res
    Ok(res.into())
  }

  /// Get customer object
  async fn get_customer(&self, r: CustomerRequest) -> ServiceResult<CustomerObj> {
    let res = self
      .commitments
      .lock()
      .await
      .find_id(&r.customer_id)?
      .unpack()
      .clone();
    Ok(res.into())
  }

  async fn has_active_commitment(
    &self,
    r: CustomerRequest,
  ) -> ServiceResult<CommitmentInfoResponse> {
    let mut customer = self
      .commitments
      .lock()
      .await
      .find_id(&r.customer_id)?
      .unpack()
      .clone();
    Ok(CommitmentInfoResponse {
      active_commitment: customer.get_active_commitment().map(|ac| ac.clone().into()),
      has_active_commitment: customer.has_active_commitment(),
    })
  }

  async fn has_active_commitment_bulk(
    &self,
    r: CustomerBulkRequest,
  ) -> ServiceResult<Vec<CommitmentInfo>> {
    let mut res: Vec<CommitmentInfo> = Vec::new();
    for customer in self
      .commitments
      .lock()
      .await
      .as_vec_mut()
      .iter_mut()
      .filter(|c| r.customer_ids.contains(&c.unpack().customer_id))
    {
      match customer.as_mut().unpack().get_active_commitment() {
        Some(c) => res.push(c.clone().into()),
        None => (),
      }
    }
    Ok(res)
  }

  async fn add_purchase(&self, r: AddPurchaseRequest) -> ServiceResult<CommitmentInfo> {
    let res = self
      .commitments
      .lock()
      .await
      .find_id_mut(&r.customer_id)?
      .as_mut()
      .unpack()
      .add_purchase(
        string_to_uuid(r.commitment_id)?,
        commitment::PurchaseInfo::new(
          string_to_uuid(r.purchase_id)?,
          r.total_net,
          r.total_gross,
          r.applied_discount,
        ),
      )
      .map_err(|e| ServiceError::bad_request(&e))?
      .clone();
    Ok(res.into())
  }

  async fn remove_purchase(&self, r: RemovePurchaseRequest) -> ServiceResult<CommitmentInfo> {
    let res = self
      .commitments
      .lock()
      .await
      .find_id_mut(&r.customer_id)?
      .as_mut()
      .unpack()
      .remove_purchase(
        string_to_uuid(r.commitment_id)?,
        &string_to_uuid(r.purchase_id)?,
      )
      .map_err(|e| ServiceError::bad_request(&e))?
      .clone();
    Ok(res.into())
  }
}

// Helper to try convert string to UUID
fn string_to_uuid(id: String) -> ServiceResult<Uuid> {
  Uuid::from_str(&id).map_err(|_| ServiceError::BadRequest(format!("A kért ID hibás: {}", id)))
}

#[tonic::async_trait]
impl Commitment for CommitmentService {
  async fn get_customer_ids(
    &self,
    request: Request<()>,
  ) -> Result<Response<proto::commitment::CustomerIds>, Status> {
    let customer_ids = self.get_customer_ids().await?;
    Ok(Response::new(CustomerIds { customer_ids }))
  }

  async fn add_commitment(
    &self,
    request: Request<proto::commitment::AddCommitmentRequest>,
  ) -> Result<Response<proto::commitment::CustomerObj>, Status> {
    let res = self.add_commitment(request.into_inner()).await?;
    Ok(Response::new(res))
  }

  async fn get_customer(
    &self,
    request: Request<proto::commitment::CustomerRequest>,
  ) -> Result<Response<proto::commitment::CustomerObj>, Status> {
    let res = self.get_customer(request.into_inner()).await?;
    Ok(Response::new(res))
  }

  async fn has_active_commitment(
    &self,
    request: Request<proto::commitment::CustomerRequest>,
  ) -> Result<Response<proto::commitment::CommitmentInfoResponse>, Status> {
    let res = self.has_active_commitment(request.into_inner()).await?;
    Ok(Response::new(res))
  }

  type HasActiveCommitmentBulkStream = ReceiverStream<Result<CommitmentInfo, Status>>;

  async fn has_active_commitment_bulk(
    &self,
    request: Request<proto::commitment::CustomerBulkRequest>,
  ) -> Result<Response<Self::HasActiveCommitmentBulkStream>, Status> {
    // Create channel for stream response
    let (mut tx, rx) = tokio::sync::mpsc::channel(100);

    // Get resources as Vec<SourceObject>
    let res = self
      .has_active_commitment_bulk(request.into_inner())
      .await?;

    // Send the result items through the channel
    tokio::spawn(async move {
      for ots in res.into_iter() {
        tx.send(Ok(ots)).await.unwrap();
      }
    });

    // Send back the receiver
    Ok(Response::new(ReceiverStream::new(rx)))
  }

  async fn add_purchase(
    &self,
    request: Request<proto::commitment::AddPurchaseRequest>,
  ) -> Result<Response<proto::commitment::CommitmentInfo>, Status> {
    let res = self.add_purchase(request.into_inner()).await?;
    Ok(Response::new(res))
  }

  async fn remove_purchase(
    &self,
    request: Request<proto::commitment::RemovePurchaseRequest>,
  ) -> Result<Response<proto::commitment::CommitmentInfo>, Status> {
    let res = self.remove_purchase(request.into_inner()).await?;
    Ok(Response::new(res))
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  // Init commitments database
  let customer_commitments: VecPack<commitment::Customer> =
    VecPack::load_or_init(PathBuf::from("data/commitments"))
      .expect("Error while loading commitments db");

  let addr = env::var("SERVICE_ADDR_COMMITMENT")
    .unwrap_or("[::1]:50074".into())
    .parse()
    .unwrap();

  // Create shutdown channel
  let (tx, rx) = oneshot::channel();

  // Spawn the server into a runtime
  tokio::task::spawn(async move {
    Server::builder()
      .add_service(CommitmentServer::new(CommitmentService::init(
        customer_commitments,
      )))
      .serve_with_shutdown(addr, async {
        let _ = rx.await;
      })
      .await
      .unwrap()
  });

  tokio::signal::ctrl_c().await?;

  println!("SIGINT");

  // Send shutdown signal after SIGINT received
  let _ = tx.send(());

  Ok(())
}
