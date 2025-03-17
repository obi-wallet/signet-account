use osmosis_std::types::osmosis::gamm::v1beta1::{
    MsgExitPool, MsgExitPoolResponse, MsgExitSwapExternAmountOut,
    MsgExitSwapExternAmountOutResponse, MsgExitSwapShareAmountIn, MsgExitSwapShareAmountInResponse,
    MsgJoinPool, MsgJoinPoolResponse, MsgJoinSwapExternAmountIn, MsgJoinSwapExternAmountInResponse,
    MsgJoinSwapShareAmountOut, MsgJoinSwapShareAmountOutResponse, MsgSwapExactAmountIn,
    MsgSwapExactAmountInResponse, MsgSwapExactAmountOut, MsgSwapExactAmountOutResponse,
};

#[uniserde::uniserde]
pub enum OsmoMsg {
    ExitPool(MsgExitPool),
    ExitPoolResponse(MsgExitPoolResponse),
    ExitSwapExternAmountOut(MsgExitSwapExternAmountOut),
    ExitSwapExternAmountOutResponse(MsgExitSwapExternAmountOutResponse),
    ExitSwapShareAmountIn(MsgExitSwapShareAmountIn),
    ExitSwapShareAmountInResponse(MsgExitSwapShareAmountInResponse),
    JoinPool(MsgJoinPool),
    JoinPoolResponse(MsgJoinPoolResponse),
    JoinSwapExternAmountIn(MsgJoinSwapExternAmountIn),
    JoinSwapExternAmountInResponse(MsgJoinSwapExternAmountInResponse),
    JoinSwapShareAmountOut(MsgJoinSwapShareAmountOut),
    JoinSwapShareAmountOutResponse(MsgJoinSwapShareAmountOutResponse),
    SwapExactAmountIn(MsgSwapExactAmountIn),
    SwapExactAmountInResponse(MsgSwapExactAmountInResponse),
    SwapExactAmountOut(MsgSwapExactAmountOut),
    SwapExactAmountOutResponse(MsgSwapExactAmountOutResponse),
}

#[cfg(feature = "cosmwasm")]
impl From<OsmoMsg> for CosmosMsg {
    fn from(val: OsmoMsg) -> Self {
        let _as_cosmos_msg: cosmwasm_std::CosmosMsg = match val {
            OsmoMsg::ExitPool(o) => o.into(),
            OsmoMsg::ExitPoolResponse(o) => o.into(),
            OsmoMsg::ExitSwapExternAmountOut(o) => o.into(),
            OsmoMsg::ExitSwapExternAmountOutResponse(o) => o.into(),
            OsmoMsg::ExitSwapShareAmountIn(o) => o.into(),
            OsmoMsg::ExitSwapShareAmountInResponse(o) => o.into(),
            OsmoMsg::JoinPool(o) => o.into(),
            OsmoMsg::JoinPoolResponse(o) => o.into(),
            OsmoMsg::JoinSwapExternAmountIn(o) => o.into(),
            OsmoMsg::JoinSwapExternAmountInResponse(o) => o.into(),
            OsmoMsg::JoinSwapShareAmountOut(o) => o.into(),
            OsmoMsg::JoinSwapShareAmountOutResponse(o) => o.into(),
            OsmoMsg::SwapExactAmountIn(o) => o.into(),
            OsmoMsg::SwapExactAmountInResponse(o) => o.into(),
            OsmoMsg::SwapExactAmountOut(o) => o.into(),
            OsmoMsg::SwapExactAmountOutResponse(o) => o.into(),
        };
        // not finished implementation yet
        CosmosMsg::Custom(Empty {})
    }
}
