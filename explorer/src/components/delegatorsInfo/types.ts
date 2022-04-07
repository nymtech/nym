export type ColumnsType = {
    field: string;
    title: string;
    headerAlign: string;
    flex?: number;
    width?: number;
    tooltipInfo?: boolean;
};

export type RowsType = {
    value: string;
    visualProgressValue?: number;
};

export interface UniversalTableProps {
    tableName: string;
    columnsData: ColumnsType[];
    rows: any[];
}