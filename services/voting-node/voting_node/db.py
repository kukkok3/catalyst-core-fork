import asyncpg
import datetime
from typing import Any, List

from .logs import getLogger
from .models import Event, HostInfo, LeaderHostInfo, Proposal
from .utils import get_hostname, LEADER_REGEX

# gets voting node logger
logger = getLogger()


class EventDb(object):
    conn: Any = None
    db_url: str

    def __init__(self, db_url: str) -> None:
        self.db_url = db_url

    async def connect(self):
        conn = await asyncpg.connect(self.db_url)
        if conn is None:
            raise Exception("failed to connect to the database")
        self.conn = conn

    async def close(self):
        if self.conn is not None:
            await self.conn.close()

    async def fetch_upcoming_event(self) -> Event:
        filter_by = "voting_start > $1"
        sort_by = "voting_start ASC"
        query = f"SELECT * FROM event WHERE {filter_by} ORDER BY {sort_by} LIMIT 1"
        now = datetime.datetime.utcnow()
        result = await self.conn.fetchrow(query, now)
        if result is None:
            raise Exception("failed to fetch event from DB")
        return Event(**dict(result))

    async def fetch_leader_host_info(self, event_row_id: int) -> HostInfo:
        filter_by = "hostname = $1 AND event = $2"
        query = f"SELECT * FROM voting_node WHERE {filter_by}"
        result = await self.conn.fetchrow(query, get_hostname(), event_row_id)
        if result is None:
            raise Exception("failed to fetch leader node info from DB")
        host_info = HostInfo(**dict(result))
        return host_info

    async def insert_leader_host_info(self, host_info: HostInfo):
        # insert the hostname row into the voting_node table
        fields = "hostname, event, seckey, pubkey, netkey"
        values = "$1, $2, $3, $4, $5"
        query = f"INSERT INTO voting_node({fields}) VALUES({values}) RETURNING *"
        h = host_info
        result = await self.conn.execute(
            query,
            h.hostname,
            h.event,
            h.seckey,
            h.pubkey,
            h.netkey,
        )
        if result is None:
            raise Exception(f"failed to insert '{h.hostname}' info to DB")
        logger.debug(f"{h.hostname} info added: {result}")

    async def fetch_leaders_host_info(self) -> List[LeaderHostInfo]:
        """Fetch host information for leader nodes.
        Returns a list of leader host information.
        Raises exceptions if the DB fails to return a list of records, or
        if the list is empty."""
        filter_by = f"hostname != $1 AND hostname ~ '{LEADER_REGEX}'"
        query = f"SELECT (hostname, pubkey) FROM voting_node WHERE {filter_by}"
        result = await self.conn.fetch(query, get_hostname())
        match result:
            case None:
                raise Exception("DB error fetching leaders host info")
            case []:
                raise Exception("no leader host info found in DB")
            case [*leaders]:

                def extract_leader_info(leader):
                    logger.debug(f"leader host info: {leader['row']}")
                    return LeaderHostInfo(*leader["row"])

                return list(map(extract_leader_info, leaders))

    async def fetch_proposals(self) -> List[Proposal]:
        query = "SELECT * FROM proposal ORDER BY id ASC"
        result = await self.conn.fetch(query)
        if result is None:
            raise Exception("proposals DB error")
        logger.debug(f"proposals retrieved from DB: {len(result)}")
        match result:
            case None:
                raise Exception("DB error fetching proposals")
            case []:
                raise Exception("no proposals found in DB")
            case [*proposals]:
                logger.debug(f"proposals retrieved from DB: {len(proposals)}")
                return list(map(lambda r: Proposal(**dict(r)), proposals))

    async def insert_block0_info(
        self, event_row_id: int, block0_bytes: bytes, block0_hash: str
    ):
        # insert the hostname row into the voting_node table
        columns = "block0 = $1, block0_hash = $2"
        condition = "row_id = $3"
        returning = "name"
        query = f"UPDATE event SET {columns} WHERE {condition} RETURNING {returning}"
        try:
            result = await self.conn.execute(
                query, block0_bytes, block0_hash, event_row_id
            )
            if result is None:
                raise Exception("failed to insert block0 info from DB")
            logger.debug(f"block0 info added to event: {result}")
        except Exception as e:
            raise Exception(f"inserting block0 info went wrong: {e}") from e
