import type { NamingConvention } from "./naming-convention";

export enum CommandComponentDataType {
  STRING = 'string',
  INTEGER = 'integer',
  FLOAT = 'float',
  BOOLEAN = 'boolean',
  KEY_VALUE_MAPPING = 'key=value',
  OPTION_LIST = 'option_list'
}

export type CommandComponentArgumentFormat = {
  description: string;
  namingConvention?: NamingConvention; // Made optional to support undefined
  examples: string[];
};

export type CommandComponentArgument = {
  name: string;
  valueDataType: CommandComponentDataType|CommandComponentDataType[];
  defaultValue?: string;
  description: string;
  required: boolean;
  formats: CommandComponentArgumentFormat[];
};

export type CommandComponentFlag = {
  longName: string;
  shortName?: string;
  valueDataType: CommandComponentDataType|CommandComponentDataType[];
  defaultValue?: string;
  description: string;
  required: boolean;
  namingConvention?: NamingConvention; // Renamed and made optional to match other interfaces
  examples?: string[];
};