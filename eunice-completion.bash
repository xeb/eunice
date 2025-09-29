#!/bin/bash
# Bash completion script for eunice
# Install: source this file or add to ~/.bashrc
# For system-wide: copy to /etc/bash_completion.d/eunice

_eunice_completion() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    # Long options
    opts="--model --prompt --config --tool-output-limit --silent --verbose --no-mcp --interact --list-models --version --help"

    # Check if we're completing after --config= or --config
    if [[ ${prev} == "--config" ]] || [[ ${cur} == --config=* ]]; then
        # Strip --config= prefix if present
        local prefix=""
        local search="${cur}"
        if [[ ${cur} == --config=* ]]; then
            prefix="--config="
            search="${cur#--config=}"
        fi

        # Generate file completions
        local files=$(compgen -f -- "${search}")
        COMPREPLY=( $(compgen -W "${files}" -- "${search}") )

        # Add prefix back if it was stripped
        if [[ -n ${prefix} ]]; then
            COMPREPLY=( "${COMPREPLY[@]/#/${prefix}}" )
        fi
        return 0
    fi

    # Check if we're completing after --prompt= or --prompt
    if [[ ${prev} == "--prompt" ]] || [[ ${cur} == --prompt=* ]]; then
        local prefix=""
        local search="${cur}"
        if [[ ${cur} == --prompt=* ]]; then
            prefix="--prompt="
            search="${cur#--prompt=}"
        fi

        local files=$(compgen -f -- "${search}")
        COMPREPLY=( $(compgen -W "${files}" -- "${search}") )

        if [[ -n ${prefix} ]]; then
            COMPREPLY=( "${COMPREPLY[@]/#/${prefix}}" )
        fi
        return 0
    fi

    # Check if we're completing after --model= or --model
    if [[ ${prev} == "--model" ]] || [[ ${cur} == --model=* ]]; then
        local models="gpt-3.5-turbo gpt-4 gpt-4o gpt-4-turbo gpt-5 chatgpt-4o-latest gemini-2.5-flash gemini-2.5-pro gemini-1.5-flash gemini-1.5-pro sonnet opus claude-sonnet claude-opus llama3.1 gpt-oss deepseek-r1"

        local prefix=""
        local search="${cur}"
        if [[ ${cur} == --model=* ]]; then
            prefix="--model="
            search="${cur#--model=}"
        fi

        COMPREPLY=( $(compgen -W "${models}" -- "${search}") )

        if [[ -n ${prefix} ]]; then
            COMPREPLY=( "${COMPREPLY[@]/#/${prefix}}" )
        fi
        return 0
    fi

    # Complete long options
    if [[ ${cur} == --* ]]; then
        COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
        return 0
    fi

    # Default: complete files for the prompt argument
    COMPREPLY=( $(compgen -f -- "${cur}") )
}

# Register completion for both eunice and eunice.py
complete -F _eunice_completion eunice
complete -F _eunice_completion eunice.py