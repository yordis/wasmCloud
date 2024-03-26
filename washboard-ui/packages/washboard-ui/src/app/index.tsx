import './_styles/index.css';

import {Home} from "lucide-react";
import {ReactElement} from 'react';
import {RouterProvider, createBrowserRouter, RouteObject} from 'react-router-dom';
import {AppLatticeClientProvider} from "./_components/app-lattice-client-provider";
import {AppProvider} from './_components/app-provider';
import {SettingsProvider} from "./_components/settings-provider.tsx";
import {AppLayout} from "./layout";
import {DashboardPage} from "./page";

const routes: RouteObject[] = [
  {
    element: <AppLayout/>,
    children: [
      {
        index: true,
        path: '/',
        element: <DashboardPage/>,
        handle: {
          breadcrumbTitle: 'Washboard',
          title: 'Washboard',
          icon: <Home/>,
        },
      },
    ],
  },
];

export function App(): ReactElement {
  return (
    <AppProvider components={[SettingsProvider, AppLatticeClientProvider]}>
      <RouterProvider router={createBrowserRouter(routes)}/>
    </AppProvider>
  );
}
