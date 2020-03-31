import {ArtemisClient, Client, GetConference, Queries} from "../../target/pkg/artemis-test/artemis_test"
import fetch from "node-fetch"

const URL = "http://localhost:8080/graphql"

let client: ArtemisClient<Queries> | null | undefined

beforeEach(() => {
    client = new Client({url: URL, fetch}) as any
})

afterEach(() => {
    client = null
})

it("returns a simple query", async () => {
    const variables = { id: "1" }
    let result = await client!.query<GetConference.ResponseData>(Queries.GetConference, variables)

    expect(result.errors).toBeUndefined()
    expect(result.data).toEqual({
        conference: {
            id: "1",
            name: "Nextbuild 2018",
            city: "Eindhoven",
            talks: [{
                id: "22",
                title: "Software Architecture for Developers",
                speakers: [{
                    name: "Simon"
                }]
            }]
        }
    })
})

it("caches the same query", async () => {
    let result = await client!.query<GetConference.ResponseData>(Queries.GetConference, { id: "1" })

    expect(result.errors).toBeUndefined()
    expect(result.data).toBeDefined()
    expect(result.data!.conference).toBeDefined()

    expect(result.debugInfo!.source).toEqual("Network")

    result = await client!.query<GetConference.ResponseData>(Queries.GetConference, { id: "1" })

    expect(result.errors).toBeUndefined()
    expect(result.data).toBeDefined()
    expect(result.data!.conference).toBeDefined()

    expect(result.debugInfo!.source).toEqual("Cache")
})

it("notifies a subscriber when the query returns", done => {
    client!.subscribe<GetConference.ResponseData>(Queries.GetConference, { id: "1" }, (ok, err) => {
        expect(err).toBeFalsy()
        expect(ok).toBeDefined()

        expect(ok!.data).toBeDefined()
        expect(ok!.data!.conference).toBeDefined()

        const conference = ok!.data!.conference!

        expect(conference.id).toEqual("1")

        done()
    })
})

it("invalidates the cache on a related query", async () => {
    let res = await client!.query<GetConference.ResponseData>(Queries.GetConference, { id: "1" })

    expect(res.errors).toBeUndefined()
    expect(res.data).toBeDefined()
    expect(res.data!.conference).toBeDefined()

    expect(res.debugInfo!.source).toEqual("Network")

    await client!.query(Queries.AddConference, { name: "test", city: "test city" })

    res = await client!.query<GetConference.ResponseData>(Queries.GetConference, { id: "1" })

    expect(res.errors).toBeUndefined()
    expect(res.data).toBeDefined()
    expect(res.data!.conference).toBeDefined()

    expect(res.debugInfo!.source).toEqual("Network")
})

it("reruns subscribed-to queries on invalidation", async (done) => {
    let calledTimes = 0
    let callback = jest.fn(() => {
        calledTimes++
        if(calledTimes > 1) done()
    })

    let firstRun = new Promise(resolve => {
        (function wait() {
            if(calledTimes > 0) return resolve()
            setTimeout(wait, 2)
        })()
    })

    client!.subscribe(Queries.GetConference, { id: "1" }, callback)
    await firstRun

    await client!.query(Queries.AddConference, { name: "test", city: "test city" })
})