#!/bin/bash

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
  -n, --name      VM name (required)
  -p, --password  Root password (required)
  -c, --cpus      Number of vCPUs (required)
  -r, --ram       RAM in MB (required); e.g. 2048, 4096, 8192
  -s, --size      Disk size to add in GB (required); e.g. 50, 100, 200
  -h, --help      Show this help message

Example:
  $0 --name myvm --password secret --cpus 4 --ram 8192 --size 100
EOF
    exit 0
}

# --- parse flags ---
VM_NAME=""
PASSWORD=""
VCPUS=""
RAM=""
SIZE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -n|--name)     VM_NAME="$2";   shift 2 ;;
        -p|--password) PASSWORD="$2";  shift 2 ;;
        -c|--cpus)     VCPUS="$2";     shift 2 ;;
        -r|--ram)      RAM="$2";       shift 2 ;;
        -s|--size)     SIZE="$2";      shift 2 ;;
        -h|--help)     usage ;;
        *)
            echo "Error: Unknown option: $1"
            usage
            ;;
    esac
done

# --- validate required flags ---
MISSING=()
[[ -z "$VM_NAME"   ]] && MISSING+=("--name")
[[ -z "$PASSWORD"  ]] && MISSING+=("--password")
[[ -z "$VCPUS"     ]] && MISSING+=("--cpus")
[[ -z "$RAM"       ]] && MISSING+=("--ram")
[[ -z "$SIZE"      ]] && MISSING+=("--size")

if [[ ${#MISSING[@]} -gt 0 ]]; then
    echo "Error: Missing required flags: ${MISSING[*]}"
    echo "Run '$0 --help' for usage."
    exit 1
fi

if [[ ! "$VCPUS" =~ ^[0-9]+$ || "$VCPUS" -lt 1 ]]; then
    echo "Error: --cpus must be a positive integer."
    exit 1
fi

if [[ ! "$RAM" =~ ^[0-9]+$ || "$RAM" -lt 256 ]]; then
    echo "Error: --ram must be a positive integer (minimum 256 MB)."
    exit 1
fi

if [[ ! "$SIZE" =~ ^[0-9]+$ || "$SIZE" -lt 1 ]]; then
    echo "Error: --size must be a positive integer in GB."
    exit 1
fi

# --- check / download base image ---
if [[ ! -f noble-server-cloudimg-amd64.img ]]; then
    echo "Base image not found. Downloading noble-server-cloudimg-amd64.img..."
    wget https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.img
    if [[ $? -ne 0 ]]; then
        echo "Error: Failed to download the base image. Exiting."
        exit 1
    fi
fi

IMAGE_PATH="/var/lib/libvirt/images/${VM_NAME}.img"

echo "Copying the base image to $IMAGE_PATH..."
cp noble-server-cloudimg-amd64.img "$IMAGE_PATH"

echo "Checking and installing guestfs-tools if needed..."
if ! dpkg -l | grep -q guestfs-tools; then
    sudo apt update && sudo apt install guestfs-tools -y
fi

echo "Setting root password inside the VM image..."
virt-customize -a "$IMAGE_PATH" --root-password password:"$PASSWORD"

echo "Resizing the image by +${SIZE}G..."
qemu-img resize "$IMAGE_PATH" +${SIZE}G

echo "Starting VM installation..."
virt-install \
    --name "$VM_NAME" \
    --ram="$RAM" \
    --vcpus="$VCPUS" \
    --cpu host \
    --hvm \
    --disk bus=virtio,path="$IMAGE_PATH" \
    --network bridge=br0 \
    --graphics none \
    --console pty,target_type=serial \
    --osinfo ubuntunoble \
    --import

echo "VM $VM_NAME has been successfully installed!"