#![cfg(test)]
use crate::contract::{NFToken, NFTokenClient};

use crate::test_util::setup_test_token;
use soroban_sdk::{
    testutils::Address as _, Address,
    Env,
};

#[test]
fn test_initialize() {
    let env = Env::default();
    let contract_id = env.register_contract(None, NFToken);
    let client = NFTokenClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);
    assert_eq!(admin, client.admin());
    // TODO: getters for other fields?
}

#[test]
fn test_mint_new() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let client = setup_test_token(&env, &admin);

    let to = Address::generate(&env);
    client.mint_new(&to);
    assert_eq!(to, client.owner(&0));
}

#[test]
fn test_approve_and_transfer_flow() {
    let env = Env::default();
    env.mock_all_auths();
    
    // Setup contract and create addresses
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);
    let client = setup_test_token(&env, &admin);
    
    // Mint a token to the owner
    client.mint_new(&owner);
    let token_id = 0;  // First token has ID 0
    
    // Verify initial ownership
    assert_eq!(client.owner(&token_id), owner);
    
    // Owner approves themselves for the token
    client.appr(&owner, &owner, &token_id);
    assert_eq!(client.get_appr(&token_id), owner);
    
    // Owner transfers token to recipient
    client.transfer(&owner, &recipient, &token_id);
    assert_eq!(client.owner(&token_id), recipient);
    
    // Recipient transfers token back to owner
    client.transfer(&recipient, &owner, &token_id);
    assert_eq!(client.owner(&token_id), owner);
    
    // Verify the token is in owner's list of owned tokens
    let owned_tokens = client.get_all_owned(&owner);
    assert_eq!(owned_tokens.len(), 1);
    assert_eq!(owned_tokens.get(0).unwrap(), token_id);
}
