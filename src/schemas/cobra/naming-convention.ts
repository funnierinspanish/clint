export type NamingConvention = {
    name: string;
    pattern: string;
    description: string;
};

export class NamingConventions {
    static String(): NamingConvention {
        return {
            name: 'String',
            pattern: '^[a-zA-Z0-9_.-]*$',
            description: 'String must start with a letter or number and can contain letters, numbers, underscores, hyphens, and periods.',
        };
    }

    static Integer(): NamingConvention {
        return {
            name: 'Integer',
            pattern: '^[0-9]+$',
            description: 'Integer must be a positive whole number.',
        };
    }

    static Float(): NamingConvention {
        return {
            name: 'Float',
            pattern: '^[0-9]+(\\.[0-9]+)?$',
            description: 'Float must be a positive number with an optional decimal point.',
        };
    }

    static ResourceName(): NamingConvention {
        return {
            name: 'Resource Name',
            pattern: '^[a-zA-Z0-9][a-zA-Z0-9_.-]*$',
            description: 'Resource name must start with a letter or number and can contain letters, numbers, underscores, hyphens, and periods.',
        };
    }

    static KeyValueMappingKey(): NamingConvention {
        return {
            name: 'Key Value Mapping Key',
            pattern: '^[a-zA-Z0-9][a-zA-Z0-9_.-]*$',
            description: 'Key must start with a letter or number and can contain letters, numbers, underscores, hyphens, and periods.',
        };
    }

    static KeyValueMappingValue(): NamingConvention {
        return {
            name: 'Key Value Mapping Value',
            pattern: '^[a-zA-Z0-9][a-zA-Z0-9_.-]*$',
            description: 'Value must start with a letter or number and can contain letters, numbers, underscores, hyphens, and periods.',
        };
    }

    static KeyValueMapping(): NamingConvention {
        return {
            name: 'Key Value Mapping',
            pattern: `${this.KeyValueMappingKey().pattern}=${this.KeyValueMappingValue().pattern}`,
            description: 'Key-Value Mapping must be in the format key=value, where key and value follow the same rules as Key and Value.',
        };
    }

    static KeyValuePathMapping(): NamingConvention {
        return {
            name: 'Key Value Path Mapping',
            pattern: `${this.KeyValueMappingKey().pattern}=${this.Path().pattern}`,
            description: 'Key/Value Path Mapping must be in the format <source_resource_path>=<destination_resource_path>.',
        };
    }

    static UUID(): NamingConvention {
        return {
            name: 'UUID',
            pattern: '^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$',
            description: 'UUID must be in the format 8-4-4-4-12 hexadecimal characters.',
        };
    }

    static Path(): NamingConvention {
        return {
            name: 'Path',
            pattern: '^(/[^/ ]*)+/?$',
            description: 'Path must start with a forward slash and can contain forward slashes and alphanumeric characters.',
        };
    }

    static URL(): NamingConvention {
        return {
            name: 'URL',
            pattern: '(^(https?|ftp)://)?([^\s/$.?#].[^\s]*$)',
            description: 'URL must start with http, https, or ftp and can contain alphanumeric characters, forward slashes, and special characters.',
        };
    }

    static GitRepository(): NamingConvention {
        return {
            name: 'Git Repository',
            pattern: '^(https?|git)://[^\s/$.?#].[^\s]*\\.git$',
            description: 'Git repository must start with http, https, or git and end with .git. Can contain alphanumeric characters, forward slashes, and special characters.',
        };
    }

    static EndOfOptionsMarker(): NamingConvention {
        return {
            name: 'End of Options Marker',
            pattern: '^\\s*--\\s*$',
            description: 'End of options marker must be exactly -- surrounded by whitespace.',
        };
    }

    static Flag(): NamingConvention {
        return {
            name: 'Flag',
            pattern: '^--?[a-zA-Z0-9][a-zA-Z0-9_.-]*$',
            description: 'Flag name must start with `-` or `--` followed by a letter or number. Can contain letters, numbers, underscores, hyphens, and periods.',
        };
    }

    static Argument(): NamingConvention {
        return {
            name: 'Argument',
            pattern: '^[a-zA-Z0-9][a-zA-Z0-9_.-]*[= ]([a-zA-Z0-9_.-]+|".*?"|\'.*?\')$',
            description: 'Argument must start with a letter or number, followed by an equals sign or a space, and then a value that follows the same rules as a string.',
        };
    }

    static OptionList(): NamingConvention {
        return {
            name: 'Option list',
            pattern: '^([0-9]+|[a-zA-Z0-9_.-]+)(,\\s*([0-9]+|[a-zA-Z0-9_.-]+))*$',
            description: 'A comma separated list of options that can be strings or integers.'
        };
    }
}