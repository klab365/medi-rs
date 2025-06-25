set quiet

# the following two commands are specific to use in a docker container or not
# if the command is run in a docker container, it will use the cargo installation inside the container (local development with docker use case)
# if the command is run outside a docker container, it will use the docker container to run the command inside the container (CI use case)
CMD := if path_exists('/.dockerenv') == "false" { 'docker run --rm -v $(pwd):/workspaces/creatorly -w /workspaces/creatorly rust-toolchain cargo' } else { 'cargo' }
CMD_TTY := if path_exists('/.dockerenv') == "false" { 'docker run -it --rm -v $(pwd):/app -w /app rust-toolchain cargo' } else { 'cargo' }

# build the docker image for ci
build-docker id:
    echo "Building Docker image..."
    docker build -t rust-toolchain -f .devcontainer/Dockerfile --build-arg UID={{ id }} .

# build the code
build *ARGS='':
    echo "Building..."
    {{ CMD }} build {{ARGS}}

# publish
publish version='0.0.0' *ARGS='':
    echo "Building for release V{{ version }}"
    # Update version in root Cargo.toml
    sed -i 's/^version = ".*"/version = "{{version}}"/' Cargo.toml
    # Update version in src-macros/Cargo.toml
    sed -i 's/^version = ".*"/version = "{{version}}"/' src-macros/Cargo.toml
    # Update dependency version for medi-rs-macros in root Cargo.toml
    sed -i 's/medi-rs-macros = { version *= *"[^"]*"/medi-rs-macros = { version = "{{version}}"/g' Cargo.toml

    {{ CMD }} publish --package medi-rs-macros --allow-dirty {{ARGS}}
    {{ CMD }} publish --package medi-rs --allow-dirty {{ARGS}}

check-format:
    echo "Checking formatting..."
    {{ CMD }} fmt --all -- --check

# format the code
format:
    echo "Formatting..."
    {{ CMD }} fmt --all

# lint the code
lint:
    echo "Linting..."
    {{ CMD }} clippy --all-targets --all-features -- -D warnings

# fix the code
fix *ARGS:
    echo "Fixing..."
    {{ CMD }} fix --allow-staged {{ARGS}}

# run the tests
test:
    echo "Testing..."
    {{ CMD }} test

# generate coverage
coverage:
    echo "Generating coverage..."
    {{ CMD }} llvm-cov --lcov --output-path lcov.info

