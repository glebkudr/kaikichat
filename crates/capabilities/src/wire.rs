use super::{Action, CapabilityError, GrantClaims, Key, Result};
use minicbor::{Decoder, Encoder, data::Type};
use std::collections::BTreeSet;

fn invalid<E>(_: E) -> CapabilityError {
    CapabilityError::InvalidGrant
}
pub fn encode(c: &GrantClaims) -> Result<Vec<u8>> {
    let mut e = Encoder::new(Vec::new());
    e.array(16)
        .map_err(invalid)?
        .u8(3)
        .map_err(invalid)?
        .bytes(&c.owner)
        .map_err(invalid)?
        .bytes(&c.agent)
        .map_err(invalid)?
        .bytes(&c.service)
        .map_err(invalid)?
        .bytes(&c.subject)
        .map_err(invalid)?;
    match c.parent {
        Some(id) => {
            e.bytes(&id).map_err(invalid)?;
        }
        None => {
            e.null().map_err(invalid)?;
        }
    }
    e.u64(c.device_epoch)
        .map_err(invalid)?
        .u64(c.service_epoch)
        .map_err(invalid)?
        .array(c.actions.len() as u64)
        .map_err(invalid)?;
    for action in &c.actions {
        e.u8(*action as u8).map_err(invalid)?;
    }
    for scope in [&c.resources, &c.recipients] {
        e.array(scope.len() as u64).map_err(invalid)?;
        for value in scope {
            e.str(value).map_err(invalid)?;
        }
    }
    e.str(&c.budget_asset)
        .map_err(invalid)?
        .u64(c.budget_units)
        .map_err(invalid)?
        .u64(c.max_data_bytes)
        .map_err(invalid)?
        .u8(c.remaining_depth)
        .map_err(invalid)?
        .bool(c.no_subcontract)
        .map_err(invalid)?;
    Ok(e.into_writer())
}
pub fn decode(body: &[u8]) -> Result<GrantClaims> {
    let mut d = Decoder::new(body);
    if d.array().map_err(invalid)? != Some(16) || d.u8().map_err(invalid)? != 3 {
        return Err(CapabilityError::InvalidGrant);
    }
    let owner = key(&mut d)?;
    let agent = key(&mut d)?;
    let service = key(&mut d)?;
    let subject = key(&mut d)?;
    let parent = if d.datatype().map_err(invalid)? == Type::Null {
        d.null().map_err(invalid)?;
        None
    } else {
        Some(key(&mut d)?)
    };
    let device_epoch = d.u64().map_err(invalid)?;
    let service_epoch = d.u64().map_err(invalid)?;
    let count = count(&mut d, 11)?;
    let mut actions = BTreeSet::new();
    for _ in 0..count {
        if !actions.insert(Action::try_from(d.u64().map_err(invalid)?)?) {
            return Err(CapabilityError::InvalidGrant);
        }
    }
    let resources = scope(&mut d)?;
    let recipients = scope(&mut d)?;
    let budget_asset = text(&mut d, 96)?;
    let claims = GrantClaims {
        owner,
        agent,
        service,
        subject,
        parent,
        device_epoch,
        service_epoch,
        actions,
        resources,
        recipients,
        budget_asset,
        budget_units: d.u64().map_err(invalid)?,
        max_data_bytes: d.u64().map_err(invalid)?,
        remaining_depth: d.u8().map_err(invalid)?,
        no_subcontract: d.bool().map_err(invalid)?,
    };
    claims.validate()?;
    if d.position() != body.len() || encode(&claims)? != body {
        return Err(CapabilityError::InvalidGrant);
    }
    Ok(claims)
}
fn key(d: &mut Decoder<'_>) -> Result<Key> {
    d.bytes().map_err(invalid)?.try_into().map_err(invalid)
}
fn count(d: &mut Decoder<'_>, max: u64) -> Result<u64> {
    d.array()
        .map_err(invalid)?
        .filter(|count| *count <= max)
        .ok_or(CapabilityError::InvalidGrant)
}
fn text(d: &mut Decoder<'_>, max: usize) -> Result<String> {
    let value = d.str().map_err(invalid)?;
    if !super::identifier(value, max) {
        return Err(CapabilityError::InvalidGrant);
    }
    Ok(value.into())
}
fn scope(d: &mut Decoder<'_>) -> Result<BTreeSet<String>> {
    let mut values = BTreeSet::new();
    for _ in 0..count(d, 32)? {
        if !values.insert(text(d, 160)?) {
            return Err(CapabilityError::InvalidGrant);
        }
    }
    Ok(values)
}
