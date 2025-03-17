import BN from "bn.js";
import {ec, eddsa} from "elliptic";
import Point = eddsa.Point;

export interface SignatureRecid {
  r: string;
  s: string;
  recid: number;
}

export function verify(secp256k1: ec, sig: SignatureRecid, y: Point, message: BN): boolean {
  const b = new BN(sig.s, 16).invm(secp256k1.n!!);
  const a = new BN(message);
  const u1 = a.mul(b).mod(secp256k1.n!!);
  const u2 = new BN(sig.r, 16).mul(b).mod(secp256k1.n!!);

  const g = secp256k1.g; // Generator point
  const gu1 = g.mul(u1);
  const yu2 = y.mul(u2);

  const resultPoint = gu1.add(yu2);
  const resultX = resultPoint.getX().mod(secp256k1.n!!);

  return new BN(sig.r, 16).eq(resultX);
}
