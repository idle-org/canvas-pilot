// Multibranch Pipeline: BRANCH_NAME is required, never guessed from the worktree.
pipeline {
    agent { label 'rust_no_docker' }

    options {
        skipDefaultCheckout(true)
        disableConcurrentBuilds()
        timeout(time: 45, unit: 'MINUTES')
    }

    environment {
        PYTHONUNBUFFERED = '1'
        PYTHONDONTWRITEBYTECODE = '1'
        NO_COLOR = '1'
        CARGO_TERM_COLOR = 'never'
        BADGE_ROOT = '/mnt/assets/coverage'
        MANUALS_ROOT = '/mnt/books/manuals'
    }

    stages {
        stage('Checkout') {
            steps {
                deleteDir()
                checkout scm
                sh 'python3 scripts/ci.py init'
            }
        }

        stage('Tools') {
            steps {
                sh '''
                    set -eu
                    python3 -c 'import sys; assert sys.version_info >= (3, 11), "Python 3.11+ required for CI"'
                    test -n "${BRANCH_NAME:?Configure this job as a Multibranch Pipeline}"
                    uname -sr
                    rustc --version
                    if ! cargo llvm-cov --version | grep -Fx 'cargo-llvm-cov 0.9.1'; then
                        cargo install cargo-llvm-cov --version 0.9.1 --locked
                    fi
                    rustup component add llvm-tools-preview clippy rustfmt
                    mdbook_version=$(cat docs/manuals/mdbook-version)
                    if ! mdbook --version 2>/dev/null | grep -Fx "mdbook v${mdbook_version}"; then
                        cargo install mdbook --version "$mdbook_version" --locked
                    fi
                    command -v rsync
                '''
            }
        }

        stage('Format, lint and docs') {
            steps {
                sh '''
                    set -eu
                    cargo fmt --all --check
                    cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
                    cargo clippy --locked -p canvas-pilot --all-targets -- -D warnings
                    RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps
                '''
            }
        }

        stage('Tests') {
            steps {
                // Keep failures visible while allowing independent coverage measurement.
                catchError(buildResult: 'FAILURE', stageResult: 'FAILURE', catchInterruptions: false) {
                    sh '''
                        set -eu
                        python3 -m unittest discover -s tests -v
                        cargo test --locked --workspace --all-features
                        cargo test --locked -p canvas-pilot
                        cargo test --locked -p canvas-pilot --no-default-features --features no_proprietary
                        cargo build --locked --workspace --all-features
                        python3 scripts/ci.py tests-passed
                    '''
                }
            }
        }
        stage('Build manuals') {
            steps { sh 'python3 scripts/manuals.py build' }
        }
        stage('Coverage') {
            steps { sh 'python3 scripts/ci.py coverage' }
        }
        stage('Publish manuals') {
            when {
                allOf {
                    branch 'main'
                    not { changeRequest() }
                    expression { currentBuild.currentResult == 'SUCCESS' }
                }
            }
            steps { sh 'python3 scripts/manuals.py publish --destination "$MANUALS_ROOT"' }
        }
    }

    post {
        cleanup {
            script {
                sh 'python3 scripts/ci.py finalize'
                archiveArtifacts artifacts: 'coverage/version.svg,coverage/test.svg,coverage/coverage.svg',
                    fingerprint: true, allowEmptyArchive: false
                if (env.BRANCH_NAME == 'main' && !env.CHANGE_ID) {
                    sh 'scripts/publish-badges.sh "$BADGE_ROOT" version test coverage'
                }
                echo "Tests, coverage diagnostics and badges finalized; result: ${currentBuild.currentResult}. No release created."
            }
        }
    }
}
