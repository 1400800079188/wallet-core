// SPDX-License-Identifier: Apache-2.0
//
// Copyright © 2017 Trust Wallet.

use crate::address::{Address, CosmosAddress};
use crate::context::CosmosContext;
use crate::modules::serializer::protobuf_serializer::SignDirectArgs;
use crate::public_key::CosmosPublicKey;
use crate::transaction::message::cosmos_generic_message::JsonRawMessage;
use crate::transaction::message::{CosmosMessage, CosmosMessageBox};
use crate::transaction::{Coin, Fee, SignMode, SignerInfo, TxBody, UnsignedTransaction};
use std::marker::PhantomData;
use std::str::FromStr;
use tw_coin_entry::coin_context::CoinContext;
use tw_coin_entry::error::{SigningError, SigningErrorType, SigningResult};
use tw_misc::traits::{OptionalEmpty, ToBytesVec};
use tw_number::U256;
use tw_proto::Cosmos::Proto;
use tw_proto::{google, serialize};

const DEFAULT_TIMEOUT_HEIGHT: u64 = 0;

pub struct TxBuilder<Context> {
    _phantom: PhantomData<Context>,
}

impl<Context> TxBuilder<Context>
where
    Context: CosmosContext,
{
    /// Please note that [`Proto::SigningInput::public_key`] must be set.
    /// If the public key should be derived from a private key, please do it before this method is called.
    pub fn unsigned_tx_from_proto(
        coin: &dyn CoinContext,
        input: &Proto::SigningInput<'_>,
    ) -> SigningResult<UnsignedTransaction<Context>> {
        let fee = input
            .fee
            .as_ref()
            .ok_or(SigningError(SigningErrorType::Error_wrong_fee))?;
        let signer = Self::signer_info_from_proto(coin, input)?;

        Ok(UnsignedTransaction {
            signer,
            fee: Self::fee_from_proto(fee)?,
            chain_id: input.chain_id.to_string(),
            account_number: input.account_number,
            tx_body: Self::tx_body_from_proto(coin, input)?,
        })
    }

    pub fn signer_info_from_proto(
        coin: &dyn CoinContext,
        input: &Proto::SigningInput,
    ) -> SigningResult<SignerInfo<Context::PublicKey>> {
        let public_key = Context::PublicKey::from_bytes(coin, &input.public_key)?;
        Ok(SignerInfo {
            public_key,
            sequence: input.sequence,
            // At this moment, we support the Direct signing mode only.
            sign_mode: SignMode::Direct,
        })
    }

    fn fee_from_proto(input: &Proto::Fee) -> SigningResult<Fee<Context::Address>> {
        let amounts = input
            .amounts
            .iter()
            .map(Self::coin_from_proto)
            .collect::<SigningResult<_>>()?;
        Ok(Fee {
            amounts,
            gas_limit: input.gas,
            payer: None,
            granter: None,
        })
    }

    fn coin_from_proto(input: &Proto::Amount<'_>) -> SigningResult<Coin> {
        let amount = U256::from_str(&input.amount)?;
        Ok(Coin {
            amount,
            denom: input.denom.to_string(),
        })
    }

    fn tx_body_from_proto(
        coin: &dyn CoinContext,
        input: &Proto::SigningInput<'_>,
    ) -> SigningResult<TxBody> {
        if input.messages.is_empty() {
            return Err(SigningError(SigningErrorType::Error_invalid_params));
        }

        let messages = input
            .messages
            .iter()
            .map(|msg| Self::tx_message(coin, msg))
            .collect::<SigningResult<_>>()?;

        Ok(TxBody {
            messages,
            memo: input.memo.to_string(),
            timeout_height: DEFAULT_TIMEOUT_HEIGHT,
        })
    }

    pub fn try_sign_direct_args(
        input: &Proto::SigningInput<'_>,
    ) -> SigningResult<Option<SignDirectArgs>> {
        use Proto::mod_Message::OneOfmessage_oneof as MessageEnum;

        let Some(msg) = input.messages.first() else {
            return Ok(None);
        };

        match msg.message_oneof {
            MessageEnum::sign_direct_message(ref direct) => Ok(Some(SignDirectArgs {
                tx_body: direct.body_bytes.to_vec(),
                auth_info: direct.auth_info_bytes.to_vec(),
                chain_id: input.chain_id.to_string(),
                account_number: input.account_number,
            })),
            _ => Ok(None),
        }
    }

    pub fn tx_message(
        coin: &dyn CoinContext,
        input: &Proto::Message,
    ) -> SigningResult<CosmosMessageBox> {
        use Proto::mod_Message::OneOfmessage_oneof as MessageEnum;

        match input.message_oneof {
            MessageEnum::send_coins_message(ref send) => Self::send_msg_from_proto(coin, send),
            MessageEnum::transfer_tokens_message(ref transfer) => {
                Self::transfer_tokens_msg_from_proto(coin, transfer)
            },
            MessageEnum::stake_message(ref delegate) => {
                Self::delegate_msg_from_proto(coin, delegate)
            },
            MessageEnum::unstake_message(ref undelegate) => {
                Self::undelegate_msg_from_proto(coin, undelegate)
            },
            MessageEnum::withdraw_stake_reward_message(ref withdraw) => {
                Self::withdraw_reward_msg_from_proto(coin, withdraw)
            },
            MessageEnum::set_withdraw_address_message(ref set) => {
                Self::set_withdraw_address_msg_from_proto(coin, set)
            },
            MessageEnum::restake_message(ref redelegate) => {
                Self::redelegate_msg_from_proto(coin, redelegate)
            },
            MessageEnum::raw_json_message(ref raw_json) => {
                Self::wasm_raw_msg_from_proto(coin, raw_json)
            },
            MessageEnum::wasm_terra_execute_contract_transfer_message(ref transfer) => {
                Self::wasm_terra_execute_contract_transfer_msg_from_proto(coin, transfer)
            },
            MessageEnum::wasm_terra_execute_contract_send_message(ref send) => {
                Self::wasm_terra_execute_contract_send_msg_from_proto(coin, send)
            },
            MessageEnum::thorchain_send_message(ref send) => {
                Self::thorchain_send_msg_from_proto(coin, send)
            },
            MessageEnum::wasm_terra_execute_contract_generic(ref generic) => {
                Self::wasm_terra_execute_contract_generic_msg_from_proto(coin, generic)
            },
            MessageEnum::wasm_execute_contract_transfer_message(ref transfer) => {
                Self::wasm_execute_contract_transfer_msg_from_proto(coin, transfer)
            },
            MessageEnum::wasm_execute_contract_send_message(ref send) => {
                Self::wasm_execute_contract_send_msg_from_proto(coin, send)
            },
            MessageEnum::wasm_execute_contract_generic(ref generic) => {
                Self::wasm_execute_contract_generic_msg_from_proto(coin, generic)
            },
            MessageEnum::sign_direct_message(ref _sign) => {
                // `SignDirect` message must be handled before this function is called.
                // Consider using `Self::try_sign_direct_args` instead.
                Err(SigningError(SigningErrorType::Error_not_supported))
            },
            MessageEnum::auth_grant(ref grant) => Self::auth_grant_msg_from_proto(coin, grant),
            MessageEnum::auth_revoke(ref revoke) => Self::auth_revoke_msg_from_proto(coin, revoke),
            MessageEnum::msg_vote(ref vote) => Self::vote_msg_from_proto(coin, vote),
            MessageEnum::msg_stride_liquid_staking_stake(ref stake) => {
                Self::stride_stake_msg_from_proto(coin, stake)
            },
            MessageEnum::msg_stride_liquid_staking_redeem(ref redeem) => {
                Self::stride_redeem_msg_from_proto(coin, redeem)
            },
            MessageEnum::thorchain_deposit_message(ref deposit) => {
                Self::thorchain_deposit_msg_from_proto(coin, deposit)
            },
            MessageEnum::None => Err(SigningError(SigningErrorType::Error_invalid_params)),
        }
    }

    pub fn send_msg_from_proto(
        coin: &dyn CoinContext,
        send: &Proto::mod_Message::Send<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::cosmos_bank_message::SendMessage;

        let amounts = send
            .amounts
            .iter()
            .map(Self::coin_from_proto)
            .collect::<SigningResult<_>>()?;
        let msg = SendMessage {
            custom_type_prefix: send.type_prefix.to_string().empty_or_some(),
            from_address: Address::from_str_with_coin(coin, &send.from_address)?,
            to_address: Address::from_str_with_coin(coin, &send.to_address)?,
            amount: amounts,
        };
        Ok(msg.into_boxed())
    }

    pub fn transfer_tokens_msg_from_proto(
        coin: &dyn CoinContext,
        transfer: &Proto::mod_Message::Transfer<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::ibc_message::{Height, TransferTokensMessage};

        let token = transfer
            .token
            .as_ref()
            .ok_or(SigningError(SigningErrorType::Error_invalid_params))?;
        let token = Self::coin_from_proto(token)?;
        let height = transfer
            .timeout_height
            .as_ref()
            .ok_or(SigningError(SigningErrorType::Error_invalid_params))?;

        let msg = TransferTokensMessage {
            source_port: transfer.source_port.to_string(),
            source_channel: transfer.source_channel.to_string(),
            token,
            sender: Address::from_str_with_coin(coin, &transfer.sender)?,
            // Don't use `Address::from_str_with_coin` as the recipient address can belong to another Cosmos chain.
            receiver: Address::from_str(&transfer.receiver)?,
            timeout_height: Height {
                revision_number: height.revision_number,
                revision_height: height.revision_height,
            },
            timeout_timestamp: transfer.timeout_timestamp,
        };
        Ok(msg.into_boxed())
    }

    pub fn delegate_msg_from_proto(
        coin: &dyn CoinContext,
        delegate: &Proto::mod_Message::Delegate<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::cosmos_staking_message::DelegateMessage;

        let amount = delegate
            .amount
            .as_ref()
            .ok_or(SigningError(SigningErrorType::Error_invalid_params))?;
        let amount = Self::coin_from_proto(amount)?;
        let msg = DelegateMessage {
            custom_type_prefix: delegate.type_prefix.to_string().empty_or_some(),
            amount,
            delegator_address: Address::from_str_with_coin(coin, &delegate.delegator_address)?,
            validator_address: Address::from_str_with_coin(coin, &delegate.validator_address)?,
        };
        Ok(msg.into_boxed())
    }

    pub fn undelegate_msg_from_proto(
        coin: &dyn CoinContext,
        undelegate: &Proto::mod_Message::Undelegate<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::cosmos_staking_message::UndelegateMessage;

        let amount = undelegate
            .amount
            .as_ref()
            .ok_or(SigningError(SigningErrorType::Error_invalid_params))?;
        let amount = Self::coin_from_proto(amount)?;

        let msg = UndelegateMessage {
            custom_type_prefix: undelegate.type_prefix.to_string().empty_or_some(),
            amount,
            delegator_address: Address::from_str_with_coin(coin, &undelegate.delegator_address)?,
            validator_address: Address::from_str_with_coin(coin, &undelegate.validator_address)?,
        };
        Ok(msg.into_boxed())
    }

    pub fn withdraw_reward_msg_from_proto(
        coin: &dyn CoinContext,
        withdraw: &Proto::mod_Message::WithdrawDelegationReward<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::cosmos_staking_message::WithdrawDelegationRewardMessage;

        let msg = WithdrawDelegationRewardMessage {
            custom_type_prefix: withdraw.type_prefix.to_string().empty_or_some(),
            delegator_address: Address::from_str_with_coin(coin, &withdraw.delegator_address)?,
            validator_address: Address::from_str_with_coin(coin, &withdraw.validator_address)?,
        };
        Ok(msg.into_boxed())
    }

    pub fn set_withdraw_address_msg_from_proto(
        coin: &dyn CoinContext,
        set: &Proto::mod_Message::SetWithdrawAddress<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::cosmos_staking_message::SetWithdrawAddressMessage;

        let msg = SetWithdrawAddressMessage {
            custom_type_prefix: set.type_prefix.to_string().empty_or_some(),
            delegator_address: Address::from_str_with_coin(coin, &set.delegator_address)?,
            withdraw_address: Address::from_str_with_coin(coin, &set.withdraw_address)?,
        };
        Ok(msg.into_boxed())
    }

    pub fn redelegate_msg_from_proto(
        coin: &dyn CoinContext,
        redelegate: &Proto::mod_Message::BeginRedelegate<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::cosmos_staking_message::BeginRedelegateMessage;

        let amount = redelegate
            .amount
            .as_ref()
            .ok_or(SigningError(SigningErrorType::Error_invalid_params))?;
        let amount = Self::coin_from_proto(amount)?;
        let validator_src_address =
            Address::from_str_with_coin(coin, &redelegate.validator_src_address)?;
        let validator_dst_address =
            Address::from_str_with_coin(coin, &redelegate.validator_dst_address)?;

        let msg = BeginRedelegateMessage {
            custom_type_prefix: redelegate.type_prefix.to_string().empty_or_some(),
            amount,
            delegator_address: Address::from_str_with_coin(coin, &redelegate.delegator_address)?,
            validator_src_address,
            validator_dst_address,
        };
        Ok(msg.into_boxed())
    }

    pub fn wasm_raw_msg_from_proto(
        _coin: &dyn CoinContext,
        raw: &Proto::mod_Message::RawJSON<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        let value = serde_json::from_str(&raw.value)
            .map_err(|_| SigningError(SigningErrorType::Error_internal))?;

        let msg = JsonRawMessage {
            msg_type: raw.type_pb.to_string(),
            value,
        };
        Ok(msg.into_boxed())
    }

    pub fn wasm_terra_execute_contract_transfer_msg_from_proto(
        coin: &dyn CoinContext,
        transfer: &Proto::mod_Message::WasmTerraExecuteContractTransfer<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::terra_wasm_message::TerraExecuteContractMessage;
        use crate::transaction::message::wasm_message::{ExecuteMsg, WasmExecutePayload};

        let execute_payload = WasmExecutePayload::Transfer {
            amount: U256::from_big_endian_slice(&transfer.amount)?,
            recipient: transfer.recipient_address.to_string(),
        };

        let msg = TerraExecuteContractMessage {
            sender: Address::from_str_with_coin(coin, &transfer.sender_address)?,
            contract: Address::from_str_with_coin(coin, &transfer.contract_address)?,
            execute_msg: ExecuteMsg::json(execute_payload)?,
            // Used in case you are sending native tokens along with this message.
            coins: Vec::default(),
        };
        Ok(msg.into_boxed())
    }

    pub fn wasm_terra_execute_contract_send_msg_from_proto(
        coin: &dyn CoinContext,
        send: &Proto::mod_Message::WasmTerraExecuteContractSend<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::terra_wasm_message::TerraExecuteContractMessage;
        use crate::transaction::message::wasm_message::{ExecuteMsg, WasmExecutePayload};

        let execute_payload = WasmExecutePayload::Send {
            amount: U256::from_big_endian_slice(&send.amount)?,
            contract: send.recipient_contract_address.to_string(),
            msg: send.msg.to_string(),
        };

        let msg = TerraExecuteContractMessage {
            sender: Address::from_str_with_coin(coin, &send.sender_address)?,
            contract: Address::from_str_with_coin(coin, &send.contract_address)?,
            execute_msg: ExecuteMsg::json(execute_payload)?,
            // Used in case you are sending native tokens along with this message.
            coins: Vec::default(),
        };
        Ok(msg.into_boxed())
    }

    pub fn wasm_terra_execute_contract_generic_msg_from_proto(
        coin: &dyn CoinContext,
        generic: &Proto::mod_Message::WasmTerraExecuteContractGeneric<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::terra_wasm_message::TerraExecuteContractMessage;
        use crate::transaction::message::wasm_message::ExecuteMsg;

        let coins = generic
            .coins
            .iter()
            .map(Self::coin_from_proto)
            .collect::<SigningResult<_>>()?;

        let msg = TerraExecuteContractMessage {
            sender: Address::from_str_with_coin(coin, &generic.sender_address)?,
            contract: Address::from_str_with_coin(coin, &generic.contract_address)?,
            execute_msg: ExecuteMsg::String(generic.execute_msg.to_string()),
            coins,
        };
        Ok(msg.into_boxed())
    }

    pub fn wasm_execute_contract_transfer_msg_from_proto(
        coin: &dyn CoinContext,
        transfer: &Proto::mod_Message::WasmExecuteContractTransfer<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::wasm_message::{
            ExecuteMsg, WasmExecuteContractMessage, WasmExecutePayload,
        };

        let transfer_payload = WasmExecutePayload::Transfer {
            amount: U256::from_big_endian_slice(&transfer.amount)?,
            recipient: transfer.recipient_address.to_string(),
        };

        let msg = WasmExecuteContractMessage {
            sender: Address::from_str_with_coin(coin, &transfer.sender_address)?,
            contract: Address::from_str_with_coin(coin, &transfer.contract_address)?,
            msg: ExecuteMsg::json(transfer_payload)?,
            // Used in case you are sending native tokens along with this message.
            coins: Vec::default(),
        };
        Ok(msg.into_boxed())
    }

    pub fn wasm_execute_contract_send_msg_from_proto(
        coin: &dyn CoinContext,
        send: &Proto::mod_Message::WasmExecuteContractSend<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::wasm_message::{
            ExecuteMsg, WasmExecuteContractMessage, WasmExecutePayload,
        };

        let execute_payload = WasmExecutePayload::Send {
            amount: U256::from_big_endian_slice(&send.amount)?,
            contract: send.recipient_contract_address.to_string(),
            msg: send.msg.to_string(),
        };

        let msg = WasmExecuteContractMessage {
            sender: Address::from_str_with_coin(coin, &send.sender_address)?,
            contract: Address::from_str_with_coin(coin, &send.contract_address)?,
            msg: ExecuteMsg::json(execute_payload)?,
            // Used in case you are sending native tokens along with this message.
            coins: Vec::default(),
        };
        Ok(msg.into_boxed())
    }

    pub fn wasm_execute_contract_generic_msg_from_proto(
        coin: &dyn CoinContext,
        generic: &Proto::mod_Message::WasmExecuteContractGeneric<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::wasm_message::{ExecuteMsg, WasmExecuteContractMessage};

        let coins = generic
            .coins
            .iter()
            .map(Self::coin_from_proto)
            .collect::<SigningResult<_>>()?;

        let msg = WasmExecuteContractMessage {
            sender: Address::from_str_with_coin(coin, &generic.sender_address)?,
            contract: Address::from_str_with_coin(coin, &generic.contract_address)?,
            msg: ExecuteMsg::String(generic.execute_msg.to_string()),
            coins,
        };
        Ok(msg.into_boxed())
    }

    pub fn thorchain_send_msg_from_proto(
        _coin: &dyn CoinContext,
        send: &Proto::mod_Message::THORChainSend<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::thorchain_message::ThorchainSendMessage;

        let amount = send
            .amounts
            .iter()
            .map(Self::coin_from_proto)
            .collect::<SigningResult<_>>()?;

        let msg = ThorchainSendMessage {
            from_address: send.from_address.to_vec(),
            to_address: send.to_address.to_vec(),
            amount,
        };
        Ok(msg.into_boxed())
    }

    pub fn auth_grant_msg_from_proto(
        coin: &dyn CoinContext,
        auth: &Proto::mod_Message::AuthGrant<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::cosmos_auth_message::AuthGrantMessage;
        use Proto::mod_Message::mod_AuthGrant::OneOfgrant_type as ProtoGrantType;

        const STAKE_AUTHORIZATION_MSG_TYPE: &str = "/cosmos.staking.v1beta1.StakeAuthorization";

        let grant_msg = match auth.grant_type {
            ProtoGrantType::grant_stake(ref stake) => google::protobuf::Any {
                type_url: STAKE_AUTHORIZATION_MSG_TYPE.to_string(),
                value: serialize(stake)
                    .map_err(|_| SigningError(SigningErrorType::Error_invalid_params))?,
            },
            ProtoGrantType::None => {
                return Err(SigningError(SigningErrorType::Error_invalid_params))
            },
        };

        let msg = AuthGrantMessage {
            granter: Address::from_str_with_coin(coin, &auth.granter)?,
            grantee: Address::from_str_with_coin(coin, &auth.grantee)?,
            grant_msg,
            expiration_secs: auth.expiration,
        };
        Ok(msg.into_boxed())
    }

    pub fn auth_revoke_msg_from_proto(
        coin: &dyn CoinContext,
        auth: &Proto::mod_Message::AuthRevoke<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::cosmos_auth_message::AuthRevokeMessage;

        let msg = AuthRevokeMessage {
            granter: Address::from_str_with_coin(coin, &auth.granter)?,
            grantee: Address::from_str_with_coin(coin, &auth.grantee)?,
            msg_type_url: auth.msg_type_url.to_string(),
        };
        Ok(msg.into_boxed())
    }

    pub fn vote_msg_from_proto(
        coin: &dyn CoinContext,
        vote: &Proto::mod_Message::MsgVote<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::cosmos_gov_message::{VoteMessage, VoteOption};
        use Proto::mod_Message::VoteOption as ProtoVoteOption;

        let option = match vote.option {
            ProtoVoteOption::_UNSPECIFIED => VoteOption::Unspecified,
            ProtoVoteOption::YES => VoteOption::Yes,
            ProtoVoteOption::ABSTAIN => VoteOption::Abstain,
            ProtoVoteOption::NO => VoteOption::No,
            ProtoVoteOption::NO_WITH_VETO => VoteOption::NoWithVeto,
        };

        let msg = VoteMessage {
            proposal_id: vote.proposal_id,
            voter: Address::from_str_with_coin(coin, &vote.voter)?,
            option,
        };
        Ok(msg.into_boxed())
    }

    pub fn stride_stake_msg_from_proto(
        coin: &dyn CoinContext,
        stake: &Proto::mod_Message::MsgStrideLiquidStakingStake<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::stride_message::StrideLiquidStakeMessage;

        let msg = StrideLiquidStakeMessage {
            creator: Address::from_str_with_coin(coin, &stake.creator)?,
            amount: U256::from_str(&stake.amount)?,
            host_denom: stake.host_denom.to_string(),
        };
        Ok(msg.into_boxed())
    }

    pub fn stride_redeem_msg_from_proto(
        _coin: &dyn CoinContext,
        redeem: &Proto::mod_Message::MsgStrideLiquidStakingRedeem<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::stride_message::StrideLiquidRedeemMessage;

        let msg = StrideLiquidRedeemMessage {
            creator: redeem.creator.to_string(),
            amount: U256::from_str(&redeem.amount)?,
            receiver: redeem.receiver.to_string(),
            host_zone: redeem.host_zone.to_string(),
        };
        Ok(msg.into_boxed())
    }

    pub fn thorchain_deposit_msg_from_proto(
        _coin: &dyn CoinContext,
        deposit: &Proto::mod_Message::THORChainDeposit<'_>,
    ) -> SigningResult<CosmosMessageBox> {
        use crate::transaction::message::thorchain_message::{
            ThorchainAsset, ThorchainCoin, ThorchainDepositMessage,
        };

        let mut coins = Vec::with_capacity(deposit.coins.len());
        for coin_proto in deposit.coins.iter() {
            let asset_proto = coin_proto
                .asset
                .as_ref()
                .ok_or(SigningError(SigningErrorType::Error_invalid_params))?;

            let asset = ThorchainAsset {
                chain: asset_proto.chain.to_string(),
                symbol: asset_proto.symbol.to_string(),
                ticker: asset_proto.ticker.to_string(),
                synth: asset_proto.synth,
            };
            coins.push(ThorchainCoin {
                asset,
                amount: U256::from_str(&coin_proto.amount)?,
                decimals: coin_proto.decimals,
            });
        }

        let msg = ThorchainDepositMessage {
            coins,
            memo: deposit.memo.to_string(),
            signer: deposit.signer.to_vec(),
        };
        Ok(msg.into_boxed())
    }
}
