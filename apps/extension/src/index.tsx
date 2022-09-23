// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import React, { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import {
  QueryClientProvider,
  QueryClient,
} from 'react-query';
import {
  MemoryRouter,
  useRoutes,
} from 'react-router-dom';
import { ChakraProvider, extendTheme, type ThemeConfig } from '@chakra-ui/react';
import { AppStateProvider, useAppState } from 'core/hooks/useAppState';
import { AccountsProvider } from 'core/hooks/useAccounts';
import { NetworksProvider } from 'core/hooks/useNetworks';
import { createStandaloneToast } from '@chakra-ui/toast';
import { routes } from 'core/routes';
import { AnalyticsProvider } from 'core/hooks/useAnalytics';
import { customColors } from 'core/colors';

const { ToastContainer } = createStandaloneToast();

// eslint-disable-next-line global-require
window.Buffer = window.Buffer || require('buffer').Buffer;

const isProductionEnv = process.env.NODE_ENV === 'production';

const theme: ThemeConfig = extendTheme({
  colors: {
    ...customColors,
  },
  initialColorMode: 'light',
  styles: {
    global: {
      'html, body': {
        margin: 0,
        overflow: isProductionEnv ? 'hidden' : undefined,
        padding: 0,
      },
    },
  },
  useSystemColorMode: false,
});

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: isProductionEnv,
    },
  },
});

function App() {
  const appRoutes = useRoutes(routes);
  const { isAppStateReady } = useAppState();

  // Pause rendering until state is ready
  return isAppStateReady ? (
    <AccountsProvider>
      <NetworksProvider>
        { appRoutes }
      </NetworksProvider>
    </AccountsProvider>
  ) : null;
}

const root = createRoot(document.getElementById('root') as Element);

// Async load development container only in development mode
const isDevelopment = (!process.env.NODE_ENV || process.env.NODE_ENV === 'development');
const devContainerPromise = isDevelopment
  ? import('core/layouts/DevelopmentContainer')
  : Promise.resolve(undefined);

devContainerPromise.then((devModule) => {
  const MaybeDevelopmentContainer = devModule?.default ?? (({ children }) => children);

  root.render(
    <StrictMode>
      <AppStateProvider>
        <QueryClientProvider client={queryClient}>
          <ChakraProvider theme={theme}>
            <MaybeDevelopmentContainer>
              <MemoryRouter>
                <AnalyticsProvider>
                  <App />
                </AnalyticsProvider>
              </MemoryRouter>
            </MaybeDevelopmentContainer>
          </ChakraProvider>
        </QueryClientProvider>
      </AppStateProvider>
      <ToastContainer />
    </StrictMode>,
  );
});
