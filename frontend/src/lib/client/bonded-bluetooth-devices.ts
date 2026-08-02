/** One device in the current Android bonded Bluetooth set. */
export type BondedBluetoothDevice = {
  address: string;
  name: string | null;
};

/** The bonded Bluetooth devices or the reason they are unavailable. */
export type BondedBluetoothDevices =
  | { status: 'unsupported' }
  | { status: 'permissionDenied' }
  | { status: 'disabled' }
  | { status: 'available'; devices: BondedBluetoothDevice[] };
