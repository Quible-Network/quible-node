#!/usr/bin/env bash

set -e
set -x

BUCKET_LOCATION=https://quible-releases.s3.amazonaws.com
LATEST_COMMIT_SHA=`curl ${BUCKET_LOCATION}/latest`
SURREAL_DEB_FILENAME=surreal_1.5.5-1_amd64.deb
PACKAGE_DEB_FILENAME=quible-node_${LATEST_COMMIT_SHA}_amd64.deb
SURREAL_DEB_URL=${BUCKET_LOCATION}/${SURREAL_DEB_FILENAME}
PACKAGE_DEB_URL=${BUCKET_LOCATION}/${PACKAGE_DEB_FILENAME}

set +x
if [ ! -f /etc/quible-signer-key ]; then
	echo "Signer key not configured. Configuring now..."
	read -p "Enter hexadecimal ECDSA signer key: " QUIBLE_SIGNER_KEY
	echo $QUIBLE_SIGNER_KEY | sudo tee /etc/quible-signer-key
fi
set -x

curl -Sso /tmp/${SURREAL_DEB_FILENAME} ${SURREAL_DEB_URL}
curl -Sso /tmp/${PACKAGE_DEB_FILENAME} ${PACKAGE_DEB_URL}

sudo dpkg -i /tmp/${SURREAL_DEB_FILENAME}
sudo dpkg -i /tmp/${PACKAGE_DEB_FILENAME}

sudo ufw allow 9013
sudo ufw allow 9014

if ! curl -Ss -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"quible_checkHealth","params":[],"id":67}' 127.0.0.1:9013; then
	set +x
	echo healthcheck failed
	exit 1
fi

set +x
EC2_PUBLIC_IP=`curl https://checkip.amazonaws.com`

echo "Local health check succeeded!"
echo "Use this command to check health from an external machine:"
cat <<EOF
  curl -Ss -H "Content-Type: application/json" -X POST --data '{"jsonrpc":"2.0","method":"quible_checkHealth","params":[],"id":67}' ${EC2_PUBLIC_IP}:9013 | grep -q healthy
EOF
