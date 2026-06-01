
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'forge' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'forge'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'forge' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a new Reticulum network project')
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Generate interface configs for hardware')
            [CompletionResult]::new('simulate', 'simulate', [CompletionResultType]::ParameterValue, 'Simulate a virtual Reticulum network')
            [CompletionResult]::new('deploy', 'deploy', [CompletionResultType]::ParameterValue, 'Deploy configs to remote nodes')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Test network config, connectivity, and policies')
            [CompletionResult]::new('monitor', 'monitor', [CompletionResultType]::ParameterValue, 'Real-time network health dashboard (TUI)')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions for bash, zsh, fish, or PowerShell')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Generate man page for forge')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'forge;init' {
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 'Network topology template')
            [CompletionResult]::new('--topology', '--topology', [CompletionResultType]::ParameterName, 'Network topology template')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'forge;generate' {
            [CompletionResult]::new('-H', '-H ', [CompletionResultType]::ParameterName, 'Hardware type (rnode-lora, serial, tcp-client, tcp-server, auto)')
            [CompletionResult]::new('--hardware', '--hardware', [CompletionResultType]::ParameterName, 'Hardware type (rnode-lora, serial, tcp-client, tcp-server, auto)')
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'Interface name (alphanumeric, hyphens, underscores)')
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'Interface name (alphanumeric, hyphens, underscores)')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format (reticulum, json, yaml)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (reticulum, json, yaml)')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('-P', '-P ', [CompletionResultType]::ParameterName, 'Hardware-specific parameters as key=value pairs, e.g. -P freq=868mhz -P bw=125khz')
            [CompletionResult]::new('--param', '--param', [CompletionResultType]::ParameterName, 'Hardware-specific parameters as key=value pairs, e.g. -P freq=868mhz -P bw=125khz')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'forge;simulate' {
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'Number of virtual nodes')
            [CompletionResult]::new('--nodes', '--nodes', [CompletionResultType]::ParameterName, 'Number of virtual nodes')
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 'Network topology (mesh, star, ring, chain)')
            [CompletionResult]::new('--topology', '--topology', [CompletionResultType]::ParameterName, 'Network topology (mesh, star, ring, chain)')
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'Simulation duration (e.g. 30s, 5m, 1h)')
            [CompletionResult]::new('--duration', '--duration', [CompletionResultType]::ParameterName, 'Simulation duration (e.g. 30s, 5m, 1h)')
            [CompletionResult]::new('-Q', '-Q ', [CompletionResultType]::ParameterName, 'Link quality (excellent, good, moderate, poor)')
            [CompletionResult]::new('--quality', '--quality', [CompletionResultType]::ParameterName, 'Link quality (excellent, good, moderate, poor)')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format (table, json, dot)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (table, json, dot)')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'forge;deploy' {
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Inventory file path (nodes.toml)')
            [CompletionResult]::new('--inventory', '--inventory', [CompletionResultType]::ParameterName, 'Inventory file path (nodes.toml)')
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Parallel deployment concurrency (max 32)')
            [CompletionResult]::new('--concurrency', '--concurrency', [CompletionResultType]::ParameterName, 'Parallel deployment concurrency (max 32)')
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 'Tag filter — only deploy nodes with this tag')
            [CompletionResult]::new('--tag', '--tag', [CompletionResultType]::ParameterName, 'Tag filter — only deploy nodes with this tag')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Config content to deploy (from file or stdin)')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format (table, json)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (table, json)')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Dry run (show what would be deployed, no remote changes)')
            [CompletionResult]::new('--provision', '--provision', [CompletionResultType]::ParameterName, 'Full provisioning (install Python, RNS, enable service)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'forge;test' {
            [CompletionResult]::new('--check', '--check', [CompletionResultType]::ParameterName, 'Check type (connectivity, latency, redundancy, policies, all)')
            [CompletionResult]::new('--threshold', '--threshold', [CompletionResultType]::ParameterName, 'Latency threshold in milliseconds')
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Config file path (forge.toml)')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Config file path (forge.toml)')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format (table, json, tap, junit)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format (table, json, tap, junit)')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'forge;monitor' {
            [CompletionResult]::new('-I', '-I ', [CompletionResultType]::ParameterName, 'Inventory file path (nodes.toml)')
            [CompletionResult]::new('--inventory', '--inventory', [CompletionResultType]::ParameterName, 'Inventory file path (nodes.toml)')
            [CompletionResult]::new('-i', '-i', [CompletionResultType]::ParameterName, 'Refresh interval in seconds')
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Refresh interval in seconds')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'forge;completions' {
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'forge;man' {
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Enable verbose logging')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress all output except errors')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'forge;help' {
            [CompletionResult]::new('init', 'init', [CompletionResultType]::ParameterValue, 'Initialize a new Reticulum network project')
            [CompletionResult]::new('generate', 'generate', [CompletionResultType]::ParameterValue, 'Generate interface configs for hardware')
            [CompletionResult]::new('simulate', 'simulate', [CompletionResultType]::ParameterValue, 'Simulate a virtual Reticulum network')
            [CompletionResult]::new('deploy', 'deploy', [CompletionResultType]::ParameterValue, 'Deploy configs to remote nodes')
            [CompletionResult]::new('test', 'test', [CompletionResultType]::ParameterValue, 'Test network config, connectivity, and policies')
            [CompletionResult]::new('monitor', 'monitor', [CompletionResultType]::ParameterValue, 'Real-time network health dashboard (TUI)')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions for bash, zsh, fish, or PowerShell')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Generate man page for forge')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'forge;help;init' {
            break
        }
        'forge;help;generate' {
            break
        }
        'forge;help;simulate' {
            break
        }
        'forge;help;deploy' {
            break
        }
        'forge;help;test' {
            break
        }
        'forge;help;monitor' {
            break
        }
        'forge;help;completions' {
            break
        }
        'forge;help;man' {
            break
        }
        'forge;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
