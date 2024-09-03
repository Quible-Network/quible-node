import { getDefaultConfig } from '@rainbow-me/rainbowkit';
import {
  arbitrum,
  base,
  mainnet,
  optimism,
  polygon,
  sepolia,
  hardhat,
} from 'wagmi/chains';

export const config = getDefaultConfig({
  appName: 'QuibleMint',
  projectId: '4b6fb046d7459d03eb3c4a5c5b87f7d8',
  chains: [
    hardhat,
    mainnet,
    polygon,
    optimism,
    arbitrum,
    base,
    ...(process.env.NEXT_PUBLIC_ENABLE_TESTNETS === 'true' ? [sepolia] : []),
  ],
  ssr: true,
});
