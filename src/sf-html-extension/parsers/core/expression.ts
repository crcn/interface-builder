import { IRange } from "sf-core/geom";

/**
 * utility function for flattening expressions
 */

export function flattenEach(fromItems: Array<BaseExpression>, toItems: Array<BaseExpression>) {
  for (const item of fromItems) {
    item._flattenDeep(toItems);
  }
}

/**
 * Generic
 */

export class BaseExpression {
  constructor(readonly type: string, public position: IRange) {

  }

  public flatten(): Array<BaseExpression> {
    const items = [];
    this._flattenDeep(items);
    return items;
  }

  public _flattenDeep(items: Array<BaseExpression>) {
    items.push(this);
  }

  public toString() {
    return "";
  }
}