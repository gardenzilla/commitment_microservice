use chrono::{DateTime, Datelike, NaiveDate, Utc};
use packman::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub trait CustomerExt
where
  Self: Sized,
{
  /// Create new customer commitment object
  fn new(
    customer_id: u32,
    target: u32,
    new_discount_percentage: u32,
    created_by: u32,
  ) -> Result<Self, String>;
  /// Add purchase to a customer commitment
  fn add_purchase(&mut self, commitment_id: Uuid, purchase: PurchaseInfo) -> Result<&Self, String>;
  /// Remove purchase from all customer commitment
  fn remove_purchase(&mut self, commitment_id: Uuid, purchase_id: &Uuid) -> Result<&Self, String>;
  /// Add new commitment
  fn add_commitment(
    &mut self,
    new_target: u32,
    new_discount_percentage: u32,
    created_by: u32,
  ) -> Result<&Self, String>;
  /// Check whether customer has a given commitment ID
  fn has_commitment(&self, commitment_id: &Uuid) -> bool;
  /// Try to get commitment as mut ref
  fn get_commitment(&mut self, commitment_id: &Uuid) -> Result<&mut Commitment, String>;
  /// Return Some(&mut Self) if there is active commitment
  fn get_active_commitment(&mut self) -> Option<&mut Commitment>;
}

pub trait CommitmentExt
where
  Self: Sized,
{
  /// Try to create new commitment
  fn new(target: u32, discount_percentage: u32, created_by: u32) -> Result<Self, String>;
  /// Try withdrawn a commitment
  /// Don't forget to add new commitment to the customers commitments
  fn withdraw(
    &mut self,
    new_target: u32,
    new_discount_percentage: u32,
    created_by: u32,
  ) -> Result<Self, String>;
  /// Add purchase info into commitment
  fn add_purchase(&mut self, purchase: PurchaseInfo) -> Result<&Self, String>;
  /// Remove purchase info from commitment
  fn remove_purchase(&mut self, purchase_id: &Uuid) -> Result<&Self, String>;
  /// true if time and withdraw ok
  fn is_active(&self) -> bool;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Customer {
  pub customer_id: u32,
  pub commitments: Vec<Commitment>,
  pub created_at: DateTime<Utc>,
  pub created_by: u32,
}

impl Default for Customer {
  fn default() -> Self {
    Self {
      customer_id: 0,
      commitments: Vec::default(),
      created_at: Utc::now(),
      created_by: 0,
    }
  }
}

impl VecPackMember for Customer {
  type Out = u32;

  fn get_id(&self) -> &Self::Out {
    &self.customer_id
  }
}

impl CustomerExt for Customer {
  fn new(
    customer_id: u32,
    target: u32,
    discount_percentage: u32,
    created_by: u32,
  ) -> Result<Self, String> {
    Ok(Self {
      customer_id,
      commitments: vec![Commitment::new(target, discount_percentage, created_by)?],
      created_at: Utc::now(),
      created_by,
    })
  }

  fn add_purchase(&mut self, commitment_id: Uuid, purchase: PurchaseInfo) -> Result<&Self, String> {
    // Check if commitment ID is under the customer
    if !self.has_commitment(&commitment_id) {
      return Err("A megadott commitment ID nem szerepel a vásárlónál!".to_string());
    }
    // Check if the required commitment is active
    match self.get_active_commitment() {
      Some(active_commitment) => match active_commitment.commitment_id == commitment_id {
        // If active_commitment is the required one
        true => {
          let _ = active_commitment.add_purchase(purchase);
          return Ok(self);
        }
        // If active commitment is not the required one
        false => return Err("A megadott commitment helyett már van újabb.".to_string()),
      },
      None => Err(
        "A megadott vásárlónak nincs aktív commitmentje, így a vásárlás nem adható hozzá."
          .to_string(),
      ),
    }
  }

  fn remove_purchase(&mut self, commitment_id: Uuid, purchase_id: &Uuid) -> Result<&Self, String> {
    // Try to get the required commitment
    let commitment = self.get_commitment(&commitment_id)?;
    // Try to remove the required purchase
    match commitment.status {
      // If its a valid commitment
      // simply remove the required purchase
      // and return self ref
      CommitmentStatus::Valid => {
        // Remove purchase
        commitment.remove_purchase(purchase_id)?;
        // Return self ref
        Ok(self)
      }
      // If its a withdrawn commitment
      // then remove the required purchase and recursively remove
      // from all of its successors
      CommitmentStatus::Withdrawn { successor } => {
        // Remove purchase from this withdrawn commitment
        let _ = commitment.remove_purchase(purchase_id);
        // And recursively remove from all of its
        // successors
        self.remove_purchase(successor, purchase_id)
      }
    }
  }

  fn add_commitment(
    &mut self,
    new_target: u32,
    new_discount_percentage: u32,
    created_by: u32,
  ) -> Result<&Self, String> {
    // Check whether we have an active to withdraw
    // or simple create a new one
    match self.get_active_commitment() {
      // If we have Some(active_commitment) then
      // try to withdraw it and add the new commitment
      Some(active_commitment) => {
        // Try to withdraw it
        let new_commitment =
          active_commitment.withdraw(new_target, new_discount_percentage, created_by)?;
        // Push new active commitment
        self.commitments.push(new_commitment);
        // Return self ref
        Ok(self)
      }
      // If None, we need to insert new commitment anyway
      // as there is no active commitment
      None => {
        self.commitments.push(Commitment::new(
          new_target,
          new_discount_percentage,
          created_by,
        )?);
        Ok(self)
      }
    }
  }

  fn get_active_commitment(&mut self) -> Option<&mut Commitment> {
    // If there is any commitment
    if let Some(last) = self.commitments.last_mut() {
      // Check if last is active
      return match last.is_active() {
        // Return mut ref if yes
        true => Some(last),
        // None if last is not active
        false => None,
      };
    }
    None
  }

  fn has_commitment(&self, commitment_id: &Uuid) -> bool {
    self
      .commitments
      .iter()
      .any(|c| c.commitment_id == *commitment_id)
  }

  fn get_commitment(&mut self, commitment_id: &Uuid) -> Result<&mut Commitment, String> {
    for c in &mut self.commitments {
      if c.commitment_id == *commitment_id {
        return Ok(c);
      }
    }
    Err("A megadott commitment ID nem található a vásárló alatt.".to_string())
  }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum CommitmentStatus {
  // Commitment should be live if date interval
  // is Ok
  Valid,
  // Commitment is withdrawn, and it has
  // a successor
  Withdrawn { successor: Uuid },
}

impl Default for CommitmentStatus {
  fn default() -> Self {
    Self::Valid
  }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Commitment {
  pub commitment_id: Uuid,             // Unique ID
  pub target: u32,                     // Target total purchase value
  pub discount_percentage: u32,        // Valid discount percentage
  pub valid_till: DateTime<Utc>,       // Commitment is valid till
  pub balance: u32,                    // Commitment balance
  pub purchase_log: Vec<PurchaseInfo>, // Purchase log
  pub status: CommitmentStatus,        // Is withdrawn because of any reason?
  pub created_at: DateTime<Utc>,       // Created at
  pub created_by: u32,                 // Created by uid
}

impl Default for Commitment {
  fn default() -> Self {
    Self {
      commitment_id: Uuid::default(),
      target: 0,
      discount_percentage: 0,
      valid_till: Utc::now(),
      balance: 0,
      purchase_log: Vec::default(),
      status: CommitmentStatus::default(),
      created_at: Utc::now(),
      created_by: 0,
    }
  }
}

impl CommitmentExt for Commitment {
  fn new(target: u32, discount_percentage: u32, created_by: u32) -> Result<Self, String> {
    match discount_percentage {
      x if x <= 6 => {
        // Define the next calendar year 1st of january.
        let valid_till_naive = NaiveDate::from_ymd(Utc::today().year() + 1, 1, 1).and_hms(0, 0, 0);
        // Build the new Commitment Object
        Ok(Self {
          commitment_id: Uuid::new_v4(),
          target,
          discount_percentage,
          valid_till: DateTime::from_utc(valid_till_naive, Utc),
          balance: 0,
          purchase_log: Vec::new(),
          status: CommitmentStatus::Valid,
          created_at: Utc::now(),
          created_by,
        })
      }
      _ => Err("A kedvezmény mértéke 0-6% között lehet!".to_string()),
    }
  }

  fn withdraw(
    &mut self,
    new_target: u32,
    new_discount_percentage: u32,
    created_by: u32,
  ) -> Result<Self, String> {
    // Try create new Commitment
    let mut new_commitment = Self::new(new_target, new_discount_percentage, created_by)?;
    // Set its status to be Withdrawn
    self.status = CommitmentStatus::Withdrawn {
      // Set successor ID to the new commitments' one
      successor: new_commitment.commitment_id.clone(),
    };
    // Set balance
    new_commitment.balance = self.balance.clone();
    // Set history
    new_commitment.purchase_log = self.purchase_log.clone();
    // Set created_at
    new_commitment.created_at = Utc::now();
    // Set created_by
    new_commitment.created_by = created_by;
    // Return new commitment
    Ok(new_commitment)
  }

  fn add_purchase(&mut self, purchase: PurchaseInfo) -> Result<&Self, String> {
    if self
      .purchase_log
      .iter()
      .any(|p| p.purchase_id == purchase.purchase_id)
    {
      return Err("A megadott vásárlás már szerepel a vásárlási előzmények között!".to_string());
    }
    self.balance += purchase.total_gross;
    self.purchase_log.push(purchase);
    Ok(self)
  }

  fn remove_purchase(&mut self, purchase_id: &Uuid) -> Result<&Self, String> {
    match self
      .purchase_log
      .iter_mut()
      .find(|pi| pi.purchase_id == *purchase_id)
    {
      Some(pi) => {
        pi.set_removed();
        self.balance -= pi.total_gross;
        Ok(self)
      }
      None => Err("A megadott vásárlási azonosító nem szerepel a kommitmentben".to_string()),
    }
  }

  fn is_active(&self) -> bool {
    match self.status {
      // If valid and date is Ok; then true; otherwise false;
      CommitmentStatus::Valid => self.valid_till >= Utc::now(),
      // If its withdrawn, its false
      CommitmentStatus::Withdrawn { successor: _ } => false,
    }
  }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PurchaseInfo {
  pub purchase_id: Uuid,
  pub total_net: u32,
  pub total_gross: u32,
  pub applied_discount: u32,
  pub removed: bool,
  pub crated_at: DateTime<Utc>,
}

impl Default for PurchaseInfo {
  fn default() -> Self {
    Self {
      purchase_id: Uuid::default(),
      total_net: 0,
      total_gross: 0,
      applied_discount: 0,
      removed: false,
      crated_at: Utc::now(),
    }
  }
}

impl PurchaseInfo {
  pub fn new(purchase_id: Uuid, total_net: u32, total_gross: u32, applied_discount: u32) -> Self {
    Self {
      purchase_id,
      total_net,
      total_gross,
      applied_discount,
      removed: false,
      crated_at: Utc::now(),
    }
  }
  pub fn set_removed(&mut self) -> &Self {
    self.removed = true;
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_commitment_percentage() {
    assert!(Commitment::new(1000, 0, 0).is_ok());
    assert!(Commitment::new(1000, 1, 0).is_ok());
    assert!(Commitment::new(1000, 2, 0).is_ok());
    assert!(Commitment::new(1000, 3, 0).is_ok());
    assert!(Commitment::new(1000, 4, 0).is_ok());
    assert!(Commitment::new(1000, 5, 0).is_ok());
    assert!(Commitment::new(1000, 6, 0).is_ok());
    assert!(Commitment::new(1000, 7, 0).is_err());
    assert!(Commitment::new(1000, 8, 0).is_err());
    assert!(Commitment::new(1000, 9, 0).is_err());
  }

  #[test]
  fn test_commitment_withdraw() {
    // Should be ok
    let mut c = Commitment::new(1000, 2, 0).unwrap();

    // Should be err
    assert!(c.remove_purchase(&Uuid::default()).is_err());

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let id3 = Uuid::new_v4();

    // Should be ok
    assert!(c
      .add_purchase(PurchaseInfo::new(id1.clone(), 100, 127, 2))
      .is_ok());
    // Should be ok
    assert!(c
      .add_purchase(PurchaseInfo::new(id2.clone(), 100, 127, 2))
      .is_ok());
    // Should be ok
    assert!(c
      .add_purchase(PurchaseInfo::new(id3.clone(), 100, 127, 2))
      .is_ok());

    // Should be ok
    assert!(c.remove_purchase(&id3).is_ok());

    let c2 = c.withdraw(1000, 0, 0).unwrap();

    assert!(!c.is_active());
  }
}
