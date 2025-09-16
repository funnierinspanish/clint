import { z } from 'zod';

// Command component data types
export enum CommandComponentDataType {
  STRING = 'string',
  INTEGER = 'integer', 
  FLOAT = 'float',
  BOOLEAN = 'boolean',
  KEY_VALUE_MAPPING = 'key=value',
  OPTION_LIST = 'option_list'
}

// Command Flag interface for generated command files
export interface CommandFlag {
  longName: string;
  shortName?: string;
  valueDataType: CommandComponentDataType | CommandComponentDataType[];
  defaultValue?: string;
  description: string;
  required: boolean;
  examples?: string[];
}

// Component type enum for usage parsing
export const ComponentTypeSchema = z.enum([
  'Flag',
  'Argument', 
  'Keyword',
  'Group',
  'AlternativeGroup',
  'KeyValuePair'
]);

// Usage component schema with recursive structure
export const UsageComponentSchema: z.ZodType<any> = z.lazy(() => z.object({
  component_type: ComponentTypeSchema,
  name: z.string(),
  required: z.boolean(),
  repeatable: z.boolean(),
  key_value: z.boolean(),
  alternatives: z.array(UsageComponentSchema),
  children: z.array(UsageComponentSchema)
}));

// Outputs schema for command execution results
export const OutputsSchema = z.object({
  stdout: z.string(),
  stderr: z.string()
});

// Flag schema
export const FlagSchema = z.object({
  short: z.string().nullable(),
  long: z.string().nullable(), 
  data_type: z.string().nullable(),
  description: z.string().nullable(),
  parent_header: z.string()
});

// Usage schema
export const UsageSchema = z.object({
  usage_string: z.string(),
  parent_header: z.string(),
  components: z.array(UsageComponentSchema).optional()
});

// Other schema for miscellaneous lines
export const OtherSchema = z.object({
  line_contents: z.string(),
  parent_header: z.string()
});

// Children schema with recursive command structure
export const ChildrenSchema: z.ZodType<any> = z.lazy(() => z.object({
  COMMAND: z.record(z.string(), CommandSchema),
  FLAG: z.array(FlagSchema),
  USAGE: z.array(UsageSchema),
  OTHER: z.array(OtherSchema)
}));

// Command schema with all fields including new depth and command_path
export const CommandSchema: z.ZodType<any> = z.lazy(() => z.object({
  name: z.string(),
  description: z.string().optional().describe('Command description - may be missing for some commands due to CLI formatting issues'),
  parent: z.string(),
  parent_header: z.string().optional(),
  depth: z.number().int().min(0).optional().describe('Nesting depth of the command (0 for root, 1 for first level, etc.)'),
  command_path: z.string().optional().describe('Full command path (e.g., "my_cli open socket all")'),
  outputs: z.record(z.string(), OutputsSchema).optional(),
  children: ChildrenSchema
}));

// Root CLI structure schema
export const CLIStructureSchema = z.object({
  name: z.string(),
  description: z.string(),
  version: z.string(),
  depth: z.number().int().min(0).optional(),
  command_path: z.string().optional(),
  children: ChildrenSchema
});

// Type exports for TypeScript usage
export type ComponentType = z.infer<typeof ComponentTypeSchema>;
export type UsageComponent = z.infer<typeof UsageComponentSchema>;
export type Outputs = z.infer<typeof OutputsSchema>;
export type Flag = z.infer<typeof FlagSchema>;
export type Usage = z.infer<typeof UsageSchema>;
export type Other = z.infer<typeof OtherSchema>;
export type Children = z.infer<typeof ChildrenSchema>;
export type Command = z.infer<typeof CommandSchema>;
export type CLIStructure = z.infer<typeof CLIStructureSchema>;

// Validation helper functions
export function validateCLIStructure(data: unknown): CLIStructure {
  return CLIStructureSchema.parse(data);
}

export function safeParseCLIStructure(data: unknown): z.SafeParseReturnType<unknown, CLIStructure> {
  return CLIStructureSchema.safeParse(data);
}

// Schema for partial validation (useful for incremental parsing)
export const PartialCommandSchema = CommandSchema.partial();
export const PartialCLIStructureSchema = CLIStructureSchema.partial();

export type PartialCommand = z.infer<typeof PartialCommandSchema>;
export type PartialCLIStructure = z.infer<typeof PartialCLIStructureSchema>;
