#!/bin/bash

# check if noble-server-cloudimg-amd64.img is in working dir - if not, wget it
if [[ ! -f noble-server-cloudimg-amd64.img ]]; then
    echo "Base image not found. Downloading noble-server-cloudimg-amd64.img..."
    wget https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.img
    if [[ $? -ne 0 ]]; then
        echo "Error: Failed to download the base image. Exiting."
        exit 1
    fi
fi

# prompt for VM_NAME
read -p "Enter VM name: " VM_NAME
if [[ -z "$VM_NAME" ]]; then
    echo "Error: VM_NAME cannot be empty. Exiting."
    exit 1
fi

# prompt for PASSWORD w silent input
read -s -p "Enter password for the VM: " PASSWORD
echo
if [[ -z "$PASSWORD" ]]; then
    echo "Error: PASSWORD cannot be empty. Exiting."
    exit 1
fi

# prompt for number of vCPUs
read -p "Enter the number of vCPUs for the VM: " VCPUS
if [[ -z "$VCPUS" || ! "$VCPUS" =~ ^[0-9]+$ ]]; then
    echo "Error: Invalid number of vCPUs. Exiting."
    exit 1
fi

# prompt for RAM size with suggestions
DEFAULT_RAM=4096
HALF_RAM=$((DEFAULT_RAM / 2))
DOUBLE_RAM=$((DEFAULT_RAM * 2))
TRIPLE_RAM=$((DEFAULT_RAM * 3))
FOUR_TIMES_RAM=$((DEFAULT_RAM * 4))
SIX_TIMES_RAM=$((DEFAULT_RAM * 6))
EIGHT_TIMES_RAM=$((DEFAULT_RAM * 8))

echo "Choose the amount of RAM for the VM:"
echo "1) $HALF_RAM MB"
echo "2) $DEFAULT_RAM MB (recommended)"
echo "3) $DOUBLE_RAM MB"
echo "4) $TRIPLE_RAM MB"
echo "5) $FOUR_TIMES_RAM MB"
echo "6) $SIX_TIMES_RAM MB"
echo "7) $EIGHT_TIMES_RAM MB"
read -p "Enter your choice (1-7) or specify a custom amount in MB: " RAM_CHOICE

case $RAM_CHOICE in
    1) RAM=$HALF_RAM ;;
    2) RAM=$DEFAULT_RAM ;;
    3) RAM=$DOUBLE_RAM ;;
    4) RAM=$TRIPLE_RAM ;;
    5) RAM=$FOUR_TIMES_RAM ;;
    6) RAM=$SIX_TIMES_RAM ;;
    7) RAM=$EIGHT_TIMES_RAM ;;
    *)
        if [[ "$RAM_CHOICE" =~ ^[0-9]+$ ]]; then
            RAM=$RAM_CHOICE
        else
            echo "Invalid choice. Exiting."
            exit 1
        fi
        ;;
esac

# define image path
IMAGE_PATH="/var/lib/libvirt/images/${VM_NAME}.img"

# copy the base image
echo "Copying the base image to $IMAGE_PATH..."
cp noble-server-cloudimg-amd64.img "$IMAGE_PATH"

# install guestfs-tools if missing
echo "Checking and installing guestfs-tools if needed..."
if ! dpkg -l | grep -q guestfs-tools; then
    sudo apt update && sudo apt install guestfs-tools -y
fi

# set root password inside the image
echo "Setting root password inside the VM image..."
virt-customize -a "$IMAGE_PATH" --root-password password:"$PASSWORD"

# resize the image
echo "Resizing the image by +100G..."
qemu-img resize "$IMAGE_PATH" +100G

# install the VM and run log in prompt
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
