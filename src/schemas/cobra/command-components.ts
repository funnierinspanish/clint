import type { NamingConvention } from "./naming-convention";

export enum CommandComponentDataType {
  STRING = 'string',
  INTEGER = 'integer',
  FLOAT = 'float',
  BOOLEAN = 'boolean',
  KEY_VALUE_MAPPING = 'key=value',
  OPTION_LIST = 'option_list'
}

export type CommandComponentArgument = {
  name: string;
  valueDataType: CommandComponentDataType|CommandComponentDataType[];
  defaultValue?: string;
  description: string;
  required: boolean;
  valueNamingConvention: NamingConvention|NamingConvention[];
  examples?: string[];
};

export type CommandComponentFlag = {
  longName: string;
  shortName?: string;
  valueDataType: CommandComponentDataType|CommandComponentDataType[];
  defaultValue?: string;
  description: string;
  required: boolean;
  valueNamingConvention: NamingConvention|NamingConvention[];
  examples?: string[];
};