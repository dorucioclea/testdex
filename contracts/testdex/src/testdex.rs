#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TopEncode, TopDecode, TypeAbi, PartialEq, Clone, Copy, Debug)]
pub enum Status {
    Funding,
    Successful
}

/// testDEX is a DEX implementing AMM
#[elrond_wasm::contract]
pub trait TestDEX {

    #[view(getLiquidityToken)]
    #[storage_mapper("liquidity_token")]
    fn liquidity_token(&self, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    #[view(getLiquidityEgld)]
    #[storage_mapper("liquidity_egld")]
    fn liquidity_egld(&self, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    #[view(getInitialK)]
    #[storage_mapper("initial_k")]
    fn initial_k(&self, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    #[view(getFee)]
    #[storage_mapper("fee")]
    fn fee(&self) -> SingleValueMapper<u32>;

    // fees of the DEX are stored here, the owner of the concract may claim these funds
    #[view(getEarnings)]
    #[storage_mapper("earnings")]
    fn earnings(&self, token: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    // constructor
    #[init]
    fn init(&self, fee: u32) {
        // values from 0 to 100
        // i.e., value 5 is 0.05 fee
        self.fee().set(&fee);
    }

    // #[endpoint(addLiquidity)]
    // #[only_owner]
    // #[payable("*")]
    // fn add_liquidity(&self, token2: &TokenIdentifier, qty: &BigUint) -> SCResult<()> {
        
    //     let (payment, token) = self.call_value().payment_token_pair();
    //     let caller = self.blockchain().get_caller();
    //     let sc_address 
        
    //     self.liquidity_token(&token).update(|liquidity_token| *liquidity_token += payment);

    //     self.send().direct(&caller, &token2, 0, &qty, &[]);

    //     self.liquidity_egld(&token2).update(|liquidity_egld| *liquidity_egld += payment);

    //     Ok(())

    // }

    #[endpoint(addLiquidityToken)]
    #[only_owner]
    #[payable("*")]
    fn add_liquidity_token(&self) -> SCResult<()> {

        let (payment, token) = self.call_value().payment_token_pair();
        let state = self.status(&token);
        
        require!(
            state == Status::Funding,
            "Pair already funded."
        );

        self.liquidity_token(&token).update(|liquidity_token| *liquidity_token += payment);

        if self.status(&token) == Status::Successful {
            let initial_k = self.calculate_k(&token);
            self.initial_k(&token).set(&initial_k);
        }

        Ok(())

    }
    
    #[endpoint(claimLiquidityToken)]
    #[only_owner]
    #[payable("*")]
    fn claim_liquidity_token(&self, token: &TokenIdentifier) -> BigUint {

        let caller = self.blockchain().get_owner_address();
        let funds = self.liquidity_token(&token).get();

        if funds > 0u32 {
            self.liquidity_token(&token).clear();
            self.send().direct(&caller, &token, 0, &funds, &[]);
        }

        funds

    }

    #[endpoint(addLiquidityEgld)]
    #[only_owner]
    #[payable("*")]
    fn add_liquidity_egld(&self, token: &TokenIdentifier) -> SCResult<()> {
        
        let payment = self.call_value().egld_value();
        let funded = self.status(&token);

        require!(
            funded == Status::Funding,
            "Pair already funded."
        );
        
        self.liquidity_egld(token).update(|liquidity_egld| *liquidity_egld += payment);

        if self.status(&token) == Status::Successful {
            let initial_k = self.calculate_k(&token);
            self.initial_k(&token).set(&initial_k);
        }

        Ok(())

    }

    #[endpoint(claimLiquidityEgld)]
    #[only_owner]
    #[payable("*")]
    fn claim_liquidity_egld(&self, token: &TokenIdentifier) -> BigUint {
        
        let caller = self.blockchain().get_owner_address();
        let funds = self.liquidity_egld(&token).get();

        if funds > 0u32 {
            self.liquidity_egld(&token).clear();
            self.send().direct(&caller, &token, 0, &funds, &[]);
        }

        funds
    }

    #[view]
    fn status(&self, token: &TokenIdentifier) -> Status {
        if self.liquidity_egld(&token).get() > 0 && self.liquidity_token(&token).get() > 0  {
            Status::Successful
        } else {
            Status::Funding
        }
    }
    
    // K va acumulando error si comprastoken con EGLD
    #[view(calculateK)]
    fn calculate_k(&self, token: &TokenIdentifier) -> BigUint {
        self.liquidity_egld(&token).get() * self.liquidity_token(&token).get()
    }

    #[endpoint(claimEarnings)]
    #[only_owner]
    #[payable("*")]
    fn claim_earnings(&self, token: &TokenIdentifier) -> BigUint {
        
        let caller = self.blockchain().get_owner_address();
        let funds = self.earnings(&token).get();

        if funds > 0u32 {
            self.earnings(&token).clear();
            self.send().direct(&caller, &token, 0, &funds, &[]);
        }

        funds

    }

    // in: qty EGLD
    // out: token
    #[view(priceEgldToken)]
    fn price_egld_token(&self, token: &TokenIdentifier, qty: &BigUint) -> BigUint {
        
        let qty_egld = self.liquidity_egld(&token).get();
        let qty_token = self.liquidity_token(&token).get();
        let fee = self.fee().get();
        let numerator: BigUint = qty_token * qty * (1000u32 - fee);
        let denominator: BigUint = qty_egld * 1000u32 + qty * (1000u32 - fee);

        numerator / denominator
    }


    // in: quantity EGLD
    // out: quantity token
    #[view(priceEgldTokenNoFee)]
    fn price_egld_token_no_fee(&self, token: &TokenIdentifier, qty: &BigUint) -> BigUint {
        
        let qty_egld = self.liquidity_egld(&token).get();
        let qty_token = self.liquidity_token(&token).get();
        let numerator: BigUint =  qty_token * 1000u32 * qty;
        let denominator: BigUint = qty_egld * 1000u32 + qty * 1000u32;

        numerator / denominator
    }

    // in: quantity EGLD
    // out: quantity token
    #[view(feeEgldToken)]
    fn fee_egld_token(&self, token: &TokenIdentifier, qty: &BigUint) -> BigUint {

        let value_fee = self.price_egld_token(&token, &qty);
        let value_no_fee = self.price_egld_token_no_fee(&token, &qty);

        value_no_fee - value_fee

    }

    #[view(priceTokenEgld)]
    fn price_token_egld(&self, token: &TokenIdentifier, qty: &BigUint) -> BigUint {
        
        let qty_egld = self.liquidity_egld(&token).get();
        let qty_token = self.liquidity_token(&token).get();
        let fee = self.fee().get();
        let numerator: BigUint = qty_egld * qty * (1000u32 - fee);
        let denominator: BigUint = qty_token * 1000u32 + qty * (1000u32 - fee);

        numerator / denominator
    }

    #[view(priceTokenEgldNumerator)]
    fn price_token_egld_numerator(&self, token: &TokenIdentifier, qty: &BigUint) -> BigUint {
        
        let qty_egld = self.liquidity_egld(&token).get();
        let fee = self.fee().get();
        let numerator: BigUint = qty_egld * qty * (1000u32 - fee);


        numerator
    }

    #[view(priceTokenEgldDenominator)]
    fn price_token_egld_denominator(&self, token: &TokenIdentifier, qty: &BigUint) -> BigUint {
        
        let qty_token = self.liquidity_token(&token).get();
        let fee = self.fee().get();
        let denominator: BigUint = qty_token * 1000u32 + qty * (1000u32 - fee);

        denominator
    }

    #[view(priceTokenEgldNoFee)]
    fn price_token_egld_no_fee(&self, token: &TokenIdentifier, qty: &BigUint) -> BigUint {

        let qty_egld = self.liquidity_egld(&token).get();
        let qty_token = self.liquidity_token(&token).get();
        let numerator: BigUint = qty_egld * 1000u32 * qty;
        let denominator: BigUint = qty_token * 1000u32 + qty * 1000u32;

        numerator / denominator

    }

    #[view(feeTokenEgld)]
    fn fee_token_egld(&self, token: &TokenIdentifier, qty: &BigUint) -> BigUint {

        let value_fee = self.price_token_egld(&token, &qty);
        let value_no_fee = self.price_token_egld_no_fee(&token, &qty);

        value_no_fee - value_fee

    }

    #[endpoint(egldToToken)]
    #[payable("*")]
    fn egld_to_token(&self, token: &TokenIdentifier) ->  SCResult<()> {
        
        let state = self.status(&token);

        require!(
            state == Status::Successful,
            "Pair still funding!"
        );


        // egld paid for token with fees
        let payment = self.call_value().egld_value(); // EGLD
        // token bought with egld with fees
        let token_fee =  self.price_egld_token(&token, &payment);
        // token bought with egld without fees
        let token_no_fee =  self.price_egld_token_no_fee(&token, &payment);
        // fees paid in token
        let earning_token = &token_no_fee - &token_fee;
        // customer's address
        let caller = self.blockchain().get_caller();
        // old K constant to to adjust the new one
        let old_k = self.calculate_k(&token);
        // let k_is_not_equal: bool = new_k != old_k;
        // let new_k_is_greater: bool = new_k - old_k > 0;

        // adjusting K constant
        // if new_k != old_k {
        //     if new_k - old_k > 0 {
        //         // I add the remaining to the earnings
        //         // maybe another option is to burn the tokens
        //         earning_token += 1u32;
        //         token_no_fee -= 1u32;
        //     }
        // }

        self.liquidity_egld(&token).update(|liquidity_egld| *liquidity_egld += &payment);
        self.liquidity_token(&token).update(|liquidity_token| *liquidity_token -= &token_no_fee);
        self.earnings(&token).update(|earnings| *earnings += &earning_token);

        let new_k = self.calculate_k(&token);
        let initial_k = self.initial_k(&token).get();
        
        // adjusting K constant
        // if new_k != initial_k {
        //     if new_k > old_k {
        //         self.liquidity_token(&token).update(|liquidity_token| *liquidity_token -= 1u32);
        //         self.earnings(&token).update(|earnings| *earnings += 1u32);
        //     } else if self.calculate_k(&token) < old_k {
        //         self.liquidity_token(&token).update(|liquidity_token| *liquidity_token += 1u32);
        //         self.earnings(&token).update(|earnings| *earnings -= 1u32);
        //     }
        // }
        // send token bought (token_fee) to customer address
        self.send().direct(&caller, &token, 0, &token_fee, &[]);

        Ok(())
    }

    #[endpoint(tokenToEgld)]
    #[payable("*")]
    fn token_to_egld(&self) -> SCResult<()> {

        let (payment, token) = self.call_value().payment_token_pair();

        let state = self.status(&token);

        require!(
            state == Status::Successful,
            "Pair still funding!"
        );


        let egld_fee =  self.price_token_egld(&token, &payment);
        let egld_no_fee =  self.price_token_egld_no_fee(&token, &payment);
        let earning_egld = &egld_no_fee - &egld_fee;
        let caller = self.blockchain().get_caller();
        let old_k = self.calculate_k(&token);

        self.liquidity_token(&token).update(|liquidity_token| *liquidity_token += &payment);
        self.liquidity_egld(&token).update(|liquidity_egld| *liquidity_egld -= &egld_no_fee);
        self.earnings(&TokenIdentifier::egld()).update(|earnings| *earnings += &earning_egld);
        
        let new_k = self.calculate_k(&token);
        let initial_k = self.initial_k(&token).get();

        self.liquidity_token(&token).update(|liquidity_token| *liquidity_token += payment);

        // adjusting K constant
        // if new_k != initial_k {
        //     if new_k > old_k {
        //         self.liquidity_egld(&token).update(|liquidity_egld| *liquidity_egld -= 1u32);
        //         self.earnings(&TokenIdentifier::egld()).update(|earnings| *earnings += 1u32);
        //     } else if self.calculate_k(&token) < old_k {
        //         self.liquidity_egld(&token).update(|liquidity_egld| *liquidity_egld += 1u32);
        //         self.earnings(&TokenIdentifier::egld()).update(|earnings| *earnings -= 1u32);
        //     }
        // }
        // send token bought (token_fee) to customer address
        self.send().direct(&caller, &TokenIdentifier::egld(), 0, &egld_fee, &[]);

        Ok(())

    }
}