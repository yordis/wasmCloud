import {ReactElement} from "react";
import {RouteObject} from "react-router-dom";

export type AppRouteObject = RouteObject & {
  handle?: {
    title?: string;
    breadcrumbTitle?: string;
    icon?: ReactElement;
    hideInMenu?: boolean;
    hideInBreadcrumb?: boolean;
  };
  children?: AppRouteObject[];
};
