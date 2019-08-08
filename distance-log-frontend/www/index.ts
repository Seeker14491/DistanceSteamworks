import {
    Grid,
    ICellRendererParams,
    GridOptions,
    ColDef
} from 'ag-grid-community';
import escapeHtml from 'escape-html';
import * as t from 'io-ts';
import moment from 'moment';
import { PathReporter } from 'io-ts/lib/PathReporter';
import { Validation } from 'io-ts';

const RawEntry = t.type({
    map_name: t.string,
    map_author: t.union([t.string, t.null]),
    map_preview: t.union([t.string, t.null]),
    mode: t.keyof({ Sprint: null, Challenge: null, Stunt: null }),
    new_recordholder: t.string,
    old_recordholder: t.union([t.string, t.null]),
    record_new: t.string,
    record_old: t.union([t.string, t.null]),
    workshop_item_id: t.union([t.string, t.null]),
    steam_id_author: t.union([t.string, t.null]),
    steam_id_new_recordholder: t.string,
    steam_id_old_recordholder: t.union([t.string, t.null]),
    fetch_time: t.string
});

interface IRawEntry extends t.TypeOf<typeof RawEntry> {}

interface GridEntry {
    fetchTime: string;
    map: mapData;
    mode: string;
    newRecordholder: PlayerData;
    previousRecordholder?: PlayerData;
    newRecord: string;
    previousRecord: string;
}

interface mapData {
    name: string;
    preview?: string;
    workshopItemId?: string;
}

interface PlayerData {
    name: string;
    steamId: string;
}

renderLocalTimestamps();

const columnDefs: ColDef[] = [
    { headerName: 'Fetch Time', field: 'fetchTime' },
    {
        headerName: 'Map',
        valueGetter: params => params.data.map.name,
        filter: true,
        cellRenderer: mapCellRenderer
    },
    {
        headerName: 'Mode',
        field: 'mode',
        filter: true,
        filterParams: { suppressAndOrCondition: true }
    },
    {
        headerName: 'New Record Holder',
        field: 'newRecordholder',
        filter: true,
        valueGetter: params => params.data.newRecordholder.name,
        cellRenderer: PlayerCellRenderer
    },
    {
        headerName: 'Previous Record Holder',
        field: 'previousRecordholder',
        filter: true,
        valueGetter: params => {
            const previousRecordholder = params.data.previousRecordholder;
            if (previousRecordholder) {
                return previousRecordholder.name;
            } else {
                return '';
            }
        },
        cellRenderer: PlayerCellRenderer
    },
    { headerName: 'New Record', field: 'newRecord' },
    { headerName: 'Previous Record', field: 'previousRecord' }
];

fetch('https://seekr.pw/distance-log/changelist.json')
    .then(response => response.json())
    .then(data => {
        const rowData = unwrapValidation(t.array(RawEntry).decode(data))
            .map(processEntry)
            .reverse();

        const gridOptions: GridOptions = {
            columnDefs: columnDefs,
            rowData: rowData,
            animateRows: true,
            onGridReady: x => x.api!.sizeColumnsToFit()
        };

        const eGridDiv = document.getElementById('grid')! as HTMLElement;
        new Grid(eGridDiv, gridOptions);

        const api = gridOptions.api!;

        api.sizeColumnsToFit();
        window.onresize = () => api.sizeColumnsToFit();

        const filterTextBox = document.getElementById(
            'filter-text-box'
        )! as HTMLInputElement;
        filterTextBox.oninput = () => api.setQuickFilter(filterTextBox.value);
    });

function processEntry(raw: IRawEntry): GridEntry {
    let fetchTime = raw.fetch_time;
    {
        const m = moment(fetchTime);
        if (m.isValid()) {
            fetchTime = m.calendar();
        }
    }

    let map: mapData = { name: raw.map_name };
    if (raw.map_preview) {
        map.preview = raw.map_preview;
    }
    if (raw.workshop_item_id) {
        map.workshopItemId = raw.workshop_item_id;
    }

    let o: GridEntry = {
        fetchTime,
        map,
        mode: raw.mode,
        newRecordholder: {
            name: raw.new_recordholder,
            steamId: raw.steam_id_new_recordholder
        },
        newRecord: raw.record_new,
        previousRecord: raw.record_old || ''
    };
    if (raw.old_recordholder && raw.steam_id_old_recordholder) {
        o.previousRecordholder = {
            name: raw.old_recordholder,
            steamId: raw.steam_id_old_recordholder
        };
    }

    return o;
}

function mapCellRenderer(
    params: ICellRendererParams & { data: { map: mapData } }
): string {
    const mapCellData = params.data.map;
    const workshopItemId = mapCellData.workshopItemId;
    let html;
    if (workshopItemId) {
        html = ` <a href="https://steamcommunity.com/sharedfiles/filedetails/?id=${workshopItemId}" target="_blank" rel="noopener noreferrer">${
            escapeHtml(mapCellData.name)
        }</a>`;
    } else {
        html = mapCellData.name + ' <b>(official level)</b>';
    }

    return html;
}

function PlayerCellRenderer(params: {
    data: GridEntry;
    colDef: { field: 'newRecordholder' | 'previousRecordholder' };
}): string {
    const playerData = params.data[params.colDef.field];
    let html;
    if (playerData) {
        html = `<a href="https://steamcommunity.com/profiles/${
            playerData.steamId
        }" target="_blank">${escapeHtml(playerData.name)}</a>`;
    } else {
        html = '';
    }

    return html;
}

function renderLocalTimestamps() {
    const list = window.document.querySelectorAll('.timestamp');

    for (let timestamp of list) {
        const m = moment(timestamp.innerHTML);
        if (m.isValid()) {
            timestamp.innerHTML = m.calendar();
        }
    }
}

function unwrapValidation<T>(result: Validation<T>): T {
    if (result.isLeft()) {
        throw PathReporter.report(result).join('\n');
    }

    return result.value;
}
