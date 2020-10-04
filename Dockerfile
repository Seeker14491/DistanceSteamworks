FROM ubuntu

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install build-essential curl gnupg lsb-release software-properties-common wget -y &&\
curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | apt-key add - &&\
echo "deb https://dl.yarnpkg.com/debian/ stable main" | tee /etc/apt/sources.list.d/yarn.list &&\
apt-get update && apt install yarn -y &&\
bash -c "$(wget -O - https://apt.llvm.org/llvm.sh)" &&\
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile minimal -y

ENV PATH="/root/.cargo/bin:$PATH"

WORKDIR /src
ENTRYPOINT [ "/src/build.sh" ]
