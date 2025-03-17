/* tslint:disable */
/* eslint-disable */
/**
*/
export function start(): void;
/**
* Compute the Keccak256 hash of a value.
* @param {any} message
* @returns {any}
*/
export function keccak256(message: any): any;
/**
* Round-based key share generator.
*/
export class KeyGenerator {
  free(): void;
/**
* Create a key generator.
* @param {any} parameters
* @param {any} party_signup
*/
  constructor(parameters: any, party_signup: any);
/**
* Handle an incoming message.
* @param {any} message
*/
  handleIncoming(message: any): void;
/**
* Proceed to the next round.
* @returns {any}
*/
  proceed(): any;
/**
* Create the key share.
* @returns {any}
*/
  create(): any;
}
/**
* Key refresh.
*/
export class KeyRefresh {
  free(): void;
/**
* Create a key refresh.
* @param {any} parameters
* @param {any} local_key
* @param {any} new_party_index
* @param {any} old_to_new
*/
  constructor(parameters: any, local_key: any, new_party_index: any, old_to_new: any);
/**
* Handle an incoming message.
* @param {any} message
*/
  handleIncoming(message: any): void;
/**
* Proceed to the next round.
* @returns {any}
*/
  proceed(): any;
/**
* Get the key share.
* @returns {any}
*/
  create(): any;
}
/**
* Round-based signing protocol.
*/
export class Signer {
  free(): void;
/**
* Create a signer.
* @param {any} index
* @param {any} participants
* @param {any} local_key
*/
  constructor(index: any, participants: any, local_key: any);
/**
* Handle an incoming message.
* @param {any} message
*/
  handleIncoming(message: any): void;
/**
* Proceed to the next round.
* @returns {any}
*/
  proceed(): any;
/**
* Returns the completed offline stage if available.
* @returns {any}
*/
  completed_offline_stage(): any;
/**
* Generate the completed offline stage and store the result
* internally to be used when `create()` is called.
*
* Return a partial signature that must be sent to the other
* signing participants.
* @param {any} message
* @returns {any}
*/
  partial(message: any): any;
/**
* Add partial signatures without validating them. Allows multiple partial signatures
* to be combined into a single partial signature before sending it to the other participants.
* @param {any} partials
* @returns {any}
*/
  add(partials: any): any;
/**
* Create and verify the signature.
* @param {any} partials
* @returns {any}
*/
  create(partials: any): any;
}
