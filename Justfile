set quiet
CMD := "cargo"

# build the docker image for ci
build-docker id:
    echo "Building Docker image..."
    docker build -t rust-toolchain -f .devcontainer/Dockerfile --build-arg UID={{ id }} .

# build the code
build:
    echo "Building..."
    {{ CMD }} build

# build for release
release version='0.0.0' *ARGS='':
    echo "Building for release V{{ version }}"
    sed -i 's/^version = ".*"$/version = "{{ version }}"/' Cargo.toml
    {{ CMD }} build --release {{ARGS}}

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
fix:
    echo "Fixing..."
    {{ CMD }} fix --allow-staged

# run the tests
test:
    echo "Testing..."
    {{ CMD }} test

# generate coverage
coverage:
    echo "Generating coverage..."
    {{ CMD }} llvm-cov --lcov --output-path lcov.info
